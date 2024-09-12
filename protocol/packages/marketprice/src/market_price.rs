use std::marker::PhantomData;

use currency::{
    self, AnyVisitor, AnyVisitorResult, Currency, CurrencyDTO, CurrencyDef, Group, MemberOf,
    SymbolStatic,
};
use finance::price::{
    dto::{with_price, PriceDTO, WithPrice},
    Price,
};
use sdk::{
    cosmwasm_std::{Addr, Storage, Timestamp},
    cw_storage_plus::Map,
};

use crate::{alarms::prefix::Prefix, config::Config, error::PriceFeedsError, feed::PriceFeed};

pub type PriceFeedBin = Vec<u8>;
pub struct PriceFeeds<'m, PriceG> {
    storage: Map<'m, (SymbolStatic, SymbolStatic), PriceFeedBin>,
    config: Config,
    _g: PhantomData<PriceG>,
}

impl<'m, PriceG> PriceFeeds<'m, PriceG>
where
    PriceG: Group,
{
    pub const fn new(namespace: &'m str, config: Config) -> Self {
        Self {
            storage: Map::new(namespace),
            config,
            _g: PhantomData,
        }
    }

    pub fn feed(
        &self,
        storage: &mut dyn Storage,
        at: Timestamp,
        sender_raw: &Addr,
        prices: &[PriceDTO<PriceG, PriceG>],
    ) -> Result<(), PriceFeedsError> {
        for price in prices {
            self.storage.update(
                storage,
                (
                    price.base().currency().first_key(),
                    price.quote().currency().first_key(),
                ),
                |feed: Option<PriceFeedBin>| -> Result<PriceFeedBin, PriceFeedsError> {
                    add_observation(
                        feed,
                        sender_raw,
                        at,
                        *price,
                        self.config.feed_valid_since(at),
                    )
                },
            )?;
        }

        Ok(())
    }

    pub fn price<'a, QuoteC, QuoteG, Iter>(
        &'m self,
        quote_c: CurrencyDTO<QuoteG>,
        storage: &'a dyn Storage,
        at: Timestamp,
        total_feeders: usize,
        leaf_to_root: Iter,
    ) -> Result<PriceDTO<PriceG, QuoteG>, PriceFeedsError>
    where
        'm: 'a,
        PriceG: Group<TopG = PriceG>,
        QuoteC: Currency + MemberOf<QuoteG> + MemberOf<PriceG>,
        QuoteG: Group + MemberOf<PriceG>,
        Iter: Iterator<Item = &'a CurrencyDTO<PriceG>> + DoubleEndedIterator,
    {
        let mut root_to_leaf = leaf_to_root.rev();
        let _root = root_to_leaf.next();
        let quote_in_price_group = quote_c.into_super_group::<PriceG>();
        debug_assert_eq!(_root, Some(&quote_in_price_group));
        PriceCollect {
            root_to_leaf,
            feeds: self,
            storage,
            at,
            total_feeders,
            c_dto: &quote_in_price_group,
            root_dto: quote_c,
            price: Price::<QuoteC, QuoteC>::identity(),
            _quote_g: PhantomData,
        }
        .do_collect()
    }

    pub fn price_of_feed<C, QuoteC>(
        &self,
        amount_c: &CurrencyDTO<PriceG>,
        quote_c: &CurrencyDTO<PriceG>,
        storage: &dyn Storage,
        at: Timestamp,
        total_feeders: usize,
    ) -> Result<Price<C, QuoteC>, PriceFeedsError>
    where
        C: Currency,
        C: MemberOf<PriceG>,
        QuoteC: Currency,
        QuoteC: MemberOf<PriceG>,
    {
        let feed_bin = self
            .storage
            .may_load(storage, (amount_c.first_key(), quote_c.first_key()))?;
        load_feed(feed_bin).and_then(|feed| feed.calc_price(&self.config, at, total_feeders))
    }
}

fn load_feed<BaseC, QuoteC>(
    feed_bin: Option<PriceFeedBin>,
) -> Result<PriceFeed<BaseC, QuoteC>, PriceFeedsError>
where
    BaseC: Currency,
    QuoteC: Currency,
{
    feed_bin.map_or_else(
        || Ok(PriceFeed::<BaseC, QuoteC>::default()),
        |bin| postcard::from_bytes(&bin).map_err(Into::into),
    )
}
struct PriceCollect<'a, 'def, Iter, QuoteC, G, QuoteQuoteC, QuoteG>
where
    Iter: Iterator<Item = &'a CurrencyDTO<G>>,
    QuoteC: Currency + MemberOf<G>,
    G: Group,
    QuoteQuoteC: Currency + MemberOf<QuoteG>,
    QuoteG: Group,
{
    root_to_leaf: Iter,
    feeds: &'a PriceFeeds<'a, G>,
    storage: &'a dyn Storage,
    at: Timestamp,
    total_feeders: usize,
    c_dto: &'def CurrencyDTO<G>,
    root_dto: CurrencyDTO<QuoteG>,
    price: Price<QuoteC, QuoteQuoteC>,
    _quote_g: PhantomData<QuoteG>,
}
impl<'a, 'def, Iter, C, G, QuoteC, QuoteG> PriceCollect<'a, 'def, Iter, C, G, QuoteC, QuoteG>
where
    Iter: Iterator<Item = &'a CurrencyDTO<G>>,
    C: Currency + MemberOf<G>,
    G: Group<TopG = G>,
    QuoteC: Currency + MemberOf<QuoteG>,
    QuoteG: Group,
{
    fn advance<'new_def, NextC>(
        self,
        accumulator: Price<NextC, QuoteC>,
        c_dto: &'new_def CurrencyDTO<G>,
    ) -> PriceCollect<'a, 'new_def, Iter, NextC, G, QuoteC, QuoteG>
    where
        'def: 'new_def,
        NextC: Currency + MemberOf<G>,
    {
        PriceCollect {
            root_to_leaf: self.root_to_leaf,
            feeds: self.feeds,
            storage: self.storage,
            at: self.at,
            total_feeders: self.total_feeders,
            c_dto,
            root_dto: self.root_dto,
            price: accumulator,
            _quote_g: self._quote_g,
        }
    }

    fn do_collect(mut self) -> Result<PriceDTO<G, QuoteG>, PriceFeedsError> {
        if let Some(next_currency) = self.root_to_leaf.next() {
            next_currency.into_currency_type(self)
        } else {
            Ok(PriceDTO::from_price(self.price, *self.c_dto, self.root_dto))
        }
    }
}
impl<'a, 'def, Iter, QuoteC, G, QuoteQuoteC, QuoteG> AnyVisitor<G>
    for PriceCollect<'a, 'def, Iter, QuoteC, G, QuoteQuoteC, QuoteG>
where
    Iter: Iterator<Item = &'a CurrencyDTO<G>>,
    QuoteC: Currency + MemberOf<G>,
    G: Group<TopG = G>,
    QuoteQuoteC: Currency + MemberOf<QuoteG>,
    QuoteG: Group,
{
    type Output = PriceDTO<G, QuoteG>;
    type Error = PriceFeedsError;

    fn on<C>(self, def: &CurrencyDTO<C::Group>) -> AnyVisitorResult<G, Self>
    where
        C: CurrencyDef,
        C::Group: MemberOf<G> + MemberOf<G::TopG>,
    {
        let next_c = def.into_super_group::<G>();
        let next_price = self.feeds.price_of_feed::<C, QuoteC>(
            &next_c,
            self.c_dto,
            self.storage,
            self.at,
            self.total_feeders,
        )?;
        let total_price = next_price * self.price;
        self.advance(total_price, &next_c).do_collect()
    }
}

fn add_observation<G, QuoteG>(
    feed_bin: Option<PriceFeedBin>,
    from: &Addr,
    at: Timestamp,
    price: PriceDTO<G, QuoteG>,
    valid_since: Timestamp,
) -> Result<PriceFeedBin, PriceFeedsError>
where
    G: Group,
    QuoteG: Group,
{
    debug_assert!(valid_since < at);
    struct AddObservation<'a, G, QuoteG> {
        feed_bin: Option<PriceFeedBin>,
        from: &'a Addr,
        at: Timestamp,
        valid_since: Timestamp,
        group: PhantomData<G>,
        quote_group: PhantomData<QuoteG>,
    }

    impl<'a, G, QuoteG> WithPrice for AddObservation<'a, G, QuoteG>
    where
        G: Group,
        QuoteG: Group,
    {
        type G = G;
        type QuoteG = QuoteG;
        type Output = PriceFeedBin;
        type Error = PriceFeedsError;

        fn exec<C, QuoteC>(self, price: Price<C, QuoteC>) -> Result<Self::Output, Self::Error>
        where
            C: Currency,
            QuoteC: Currency,
        {
            load_feed(self.feed_bin).and_then(|feed| {
                let feed =
                    feed.add_observation(self.from.clone(), self.at, price, self.valid_since);
                postcard::to_allocvec(&feed).map_err(Into::into)
            })
        }
    }
    with_price::execute(
        price,
        AddObservation {
            feed_bin,
            from,
            at,
            valid_since,
            group: PhantomData,
            quote_group: PhantomData,
        },
    )
}

#[cfg(test)]
mod test {
    use currency::test::{
        SubGroup, SubGroupTestC10, SuperGroup, SuperGroupTestC1, SuperGroupTestC2,
        SuperGroupTestC3, SuperGroupTestC4, SuperGroupTestC5,
    };
    use currency::{Group, MemberOf};
    use finance::{
        coin::Coin,
        duration::Duration,
        percent::Percent,
        price::{self, dto::PriceDTO, Price},
    };
    use sdk::cosmwasm_std::{Addr, MemoryStorage, Timestamp};

    use crate::{error::PriceFeedsError, market_price::Config};

    use super::PriceFeeds;

    const FEEDS_NAMESPACE: &str = "feeds";
    const FEEDER: &str = "0xifeege";
    const TOTAL_FEEDERS: usize = 1;
    const FEED_VALIDITY: Duration = Duration::from_secs(30);
    const SAMPLE_PERIOD_SECS: Duration = Duration::from_secs(5);
    const SAMPLES_NUMBER: u16 = 6;
    const DISCOUNTING_FACTOR: Percent = Percent::from_permille(750);

    const NOW: Timestamp = Timestamp::from_seconds(FEED_VALIDITY.secs() * 2);

    #[test]
    fn no_feed() {
        let feeds = PriceFeeds::<SuperGroup>::new(FEEDS_NAMESPACE, config());
        let storage = MemoryStorage::new();

        assert_eq!(
            Ok(Price::<SuperGroupTestC1, SuperGroupTestC1>::identity().into()),
            feeds.price::<SuperGroupTestC1, SuperGroup, _>(
                currency::dto::<SuperGroupTestC1, _>(),
                &storage,
                NOW,
                TOTAL_FEEDERS,
                [&currency::dto::<SuperGroupTestC1, _>(),].into_iter()
            )
        );

        assert_eq!(
            Err(PriceFeedsError::NoPrice()),
            feeds.price::<SuperGroupTestC1, SuperGroup, _>(
                currency::dto::<SuperGroupTestC1, _>(),
                &storage,
                NOW,
                TOTAL_FEEDERS,
                [
                    &currency::dto::<SuperGroupTestC5, _>(),
                    &currency::dto::<SuperGroupTestC1, _>(),
                ]
                .into_iter()
            )
        );
    }

    #[test]
    fn feed_pair() {
        fn build_price<QuoteG>() -> PriceDTO<SuperGroup, QuoteG>
        where
            SubGroup: MemberOf<QuoteG>,
            QuoteG: Group,
        {
            price::total_of(Coin::<SuperGroupTestC5>::new(1))
                .is(Coin::<SubGroupTestC10>::new(18500))
                .into()
        }

        let feeds = PriceFeeds::new(FEEDS_NAMESPACE, config());
        let mut storage = MemoryStorage::new();

        feeds
            .feed(
                &mut storage,
                NOW,
                &Addr::unchecked(FEEDER),
                &[build_price()],
            )
            .unwrap();

        assert_eq!(
            Err(PriceFeedsError::NoPrice()),
            feeds.price::<SuperGroupTestC1, SuperGroup, _>(
                currency::dto::<SuperGroupTestC1, _>(),
                &storage,
                NOW,
                TOTAL_FEEDERS,
                [
                    &currency::dto::<SuperGroupTestC5, _>(),
                    &currency::dto::<SuperGroupTestC1, _>(),
                ]
                .into_iter()
            )
        );
        assert_eq!(
            Ok(build_price()),
            feeds.price::<SubGroupTestC10, SubGroup, _>(
                currency::dto::<SubGroupTestC10, _>(),
                &storage,
                NOW,
                TOTAL_FEEDERS,
                [
                    &currency::dto::<SuperGroupTestC5, _>(),
                    &currency::dto::<SubGroupTestC10, _>(),
                ]
                .into_iter()
            )
        );
    }

    #[test]
    fn feed_pairs() {
        let feeds = PriceFeeds::<SuperGroup>::new(FEEDS_NAMESPACE, config());
        let mut storage = MemoryStorage::new();
        let new_price75: Price<SuperGroupTestC5, SuperGroupTestC3> =
            price::total_of(Coin::new(1)).is(Coin::new(2));
        let new_price56 =
            price::total_of(Coin::<SuperGroupTestC3>::new(1)).is(Coin::<SuperGroupTestC4>::new(3));
        let new_price51 =
            price::total_of(Coin::<SuperGroupTestC3>::new(1)).is(Coin::<SubGroupTestC10>::new(4));

        feeds
            .feed(
                &mut storage,
                NOW,
                &Addr::unchecked(FEEDER),
                &[new_price51.into(), new_price75.into(), new_price56.into()],
            )
            .unwrap();

        assert_eq!(
            Err(PriceFeedsError::NoPrice()),
            feeds.price::<SuperGroupTestC2, SuperGroup, _>(
                currency::dto::<SuperGroupTestC2, _>(),
                &storage,
                NOW,
                TOTAL_FEEDERS,
                [
                    &currency::dto::<SuperGroupTestC5, _>(),
                    &currency::dto::<SuperGroupTestC2, _>(),
                ]
                .into_iter()
            )
        );
        assert_eq!(
            Ok(new_price75.into()),
            feeds.price::<SuperGroupTestC3, SuperGroup, _>(
                currency::dto::<SuperGroupTestC3, _>(),
                &storage,
                NOW,
                TOTAL_FEEDERS,
                [
                    &currency::dto::<SuperGroupTestC5, _>(),
                    &currency::dto::<SuperGroupTestC3, _>(),
                ]
                .into_iter()
            )
        );
        assert_eq!(
            Ok(new_price56.into()),
            feeds.price::<SuperGroupTestC4, SuperGroup, _>(
                currency::dto::<SuperGroupTestC4, _>(),
                &storage,
                NOW,
                TOTAL_FEEDERS,
                [
                    &currency::dto::<SuperGroupTestC3, _>(),
                    &currency::dto::<SuperGroupTestC4, _>(),
                ]
                .into_iter()
            )
        );
        assert_eq!(
            Ok(new_price51.into()),
            feeds.price::<SubGroupTestC10, SubGroup, _>(
                currency::dto::<SubGroupTestC10, _>(),
                &storage,
                NOW,
                TOTAL_FEEDERS,
                [
                    &currency::dto::<SuperGroupTestC3, _>(),
                    &currency::dto::<SubGroupTestC10, _>(),
                ]
                .into_iter()
            )
        );
        assert_eq!(
            Ok((new_price75 * new_price56).into()),
            feeds.price::<SuperGroupTestC4, SuperGroup, _>(
                currency::dto::<SuperGroupTestC4, _>(),
                &storage,
                NOW,
                TOTAL_FEEDERS,
                [
                    &currency::dto::<SuperGroupTestC5, _>(),
                    &currency::dto::<SuperGroupTestC3, _>(),
                    &currency::dto::<SuperGroupTestC4, _>(),
                ]
                .into_iter()
            )
        );
        assert_eq!(
            Ok((new_price75 * new_price51).into()),
            feeds.price::<SubGroupTestC10, SubGroup, _>(
                currency::dto::<SubGroupTestC10, _>(),
                &storage,
                NOW,
                TOTAL_FEEDERS,
                [
                    &currency::dto::<SuperGroupTestC5, _>(),
                    &currency::dto::<SuperGroupTestC3, _>(),
                    &currency::dto::<SubGroupTestC10, _>(),
                ]
                .into_iter()
            )
        );
    }

    fn config() -> Config {
        Config::new(
            Percent::HUNDRED,
            SAMPLE_PERIOD_SECS,
            SAMPLES_NUMBER,
            DISCOUNTING_FACTOR,
        )
    }
}
