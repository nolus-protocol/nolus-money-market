use std::marker::PhantomData;

use currency::{
    self, AnyVisitor, AnyVisitorResult, Currency, Group, GroupVisit, MemberOf, SymbolOwned,
    SymbolSlice, Tickers,
};
use finance::price::{
    dto::{with_price, PriceDTO, WithPrice},
    Price,
};
use sdk::{
    cosmwasm_std::{Addr, Storage, Timestamp},
    cw_storage_plus::Map,
};

use crate::{config::Config, error::PriceFeedsError, feed::PriceFeed};

pub type PriceFeedBin = Vec<u8>;
pub struct PriceFeeds<'m, G> {
    storage: Map<'m, (SymbolOwned, SymbolOwned), PriceFeedBin>,
    config: Config,
    _g: PhantomData<G>,
}

impl<'m, G> PriceFeeds<'m, G>
where
    G: Group,
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
        prices: &[PriceDTO<G, G>],
    ) -> Result<(), PriceFeedsError> {
        for price in prices {
            self.storage.update(
                storage,
                (
                    price.base().ticker().to_string(),
                    price.quote().ticker().to_string(),
                ),
                |feed: Option<PriceFeedBin>| -> Result<PriceFeedBin, PriceFeedsError> {
                    add_observation(
                        feed,
                        sender_raw,
                        at,
                        price,
                        self.config.feed_valid_since(at),
                    )
                },
            )?;
        }

        Ok(())
    }

    pub fn price<'a, QuoteC, QuoteG, Iter>(
        &'m self,
        storage: &'a dyn Storage,
        at: Timestamp,
        total_feeders: usize,
        leaf_to_root: Iter,
    ) -> Result<PriceDTO<G, QuoteG>, PriceFeedsError>
    where
        'm: 'a,
        G: Group,
        QuoteC: Currency + MemberOf<QuoteG> + MemberOf<G>,
        QuoteG: Group,
        Iter: Iterator<Item = &'a SymbolSlice> + DoubleEndedIterator,
    {
        let mut root_to_leaf = leaf_to_root.rev();
        let _root = root_to_leaf.next();
        debug_assert_eq!(_root, Some(QuoteC::TICKER));
        PriceCollect::do_collect(
            root_to_leaf,
            self,
            storage,
            at,
            total_feeders,
            Price::<QuoteC, QuoteC>::identity(),
        )
    }

    pub fn price_of_feed<C, QuoteC>(
        &self,
        storage: &dyn Storage,
        at: Timestamp,
        total_feeders: usize,
    ) -> Result<Price<C, QuoteC>, PriceFeedsError>
    where
        C: Currency,
        QuoteC: Currency,
    {
        let feed_bin = self
            .storage
            .may_load(storage, (C::TICKER.into(), QuoteC::TICKER.into()))?;
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
struct PriceCollect<'a, Iter, C, G, QuoteC, QuoteG>
where
    Iter: Iterator<Item = &'a SymbolSlice>,
    C: Currency + MemberOf<G>,
    G: Group,
    QuoteC: Currency + MemberOf<QuoteG>,
    QuoteG: Group,
{
    currency_path: Iter,
    feeds: &'a PriceFeeds<'a, G>,
    storage: &'a dyn Storage,
    at: Timestamp,
    total_feeders: usize,
    price: Price<C, QuoteC>,
    _quote_g: PhantomData<QuoteG>,
}
impl<'a, Iter, C, G, QuoteC, QuoteG> PriceCollect<'a, Iter, C, G, QuoteC, QuoteG>
where
    Iter: Iterator<Item = &'a SymbolSlice>,
    C: Currency + MemberOf<G>,
    G: Group,
    QuoteC: Currency + MemberOf<QuoteG>,
    QuoteG: Group,
{
    fn do_collect(
        mut currency_path: Iter,
        feeds: &'a PriceFeeds<'a, G>,
        storage: &'a dyn Storage,
        at: Timestamp,
        total_feeders: usize,
        price: Price<C, QuoteC>,
    ) -> Result<PriceDTO<G, QuoteG>, PriceFeedsError> {
        if let Some(next_currency) = currency_path.next() {
            let next_collect = PriceCollect {
                currency_path,
                feeds,
                storage,
                at,
                total_feeders,
                price,
                _quote_g: PhantomData,
            };
            Tickers::visit_any(next_currency, next_collect)
        } else {
            Ok(price.into())
        }
    }
}
impl<'a, Iter, QuoteC, G, QuoteQuoteC, QuoteG> AnyVisitor<G>
    for PriceCollect<'a, Iter, QuoteC, G, QuoteQuoteC, QuoteG>
where
    Iter: Iterator<Item = &'a SymbolSlice>,
    QuoteC: Currency + MemberOf<G>,
    G: Group,
    QuoteQuoteC: Currency + MemberOf<QuoteG>,
    QuoteG: Group,
{
    type VisitorG = G;
    type Output = PriceDTO<G, QuoteG>;
    type Error = PriceFeedsError;

    fn on<C>(self) -> AnyVisitorResult<G, Self>
    where
        C: Currency + MemberOf<Self::VisitorG>,
    {
        let next_price =
            self.feeds
                .price_of_feed::<C, _>(self.storage, self.at, self.total_feeders)?;
        let total_price = next_price * self.price;
        PriceCollect::do_collect(
            self.currency_path,
            self.feeds,
            self.storage,
            self.at,
            self.total_feeders,
            total_price,
        )
    }
}

fn add_observation<G, QuoteG>(
    feed_bin: Option<PriceFeedBin>,
    from: &Addr,
    at: Timestamp,
    price: &PriceDTO<G, QuoteG>,
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
        SubGroup, SubGroupTestC1, SuperGroup, SuperGroupTestC1, SuperGroupTestC2, SuperGroupTestC3,
        SuperGroupTestC4, SuperGroupTestC5,
    };
    use currency::{Currency, Group, MemberOf};
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
                &storage,
                NOW,
                TOTAL_FEEDERS,
                [SuperGroupTestC1::TICKER].into_iter()
            )
        );

        assert_eq!(
            Err(PriceFeedsError::NoPrice()),
            feeds.price::<SuperGroupTestC1, SuperGroup, _>(
                &storage,
                NOW,
                TOTAL_FEEDERS,
                [SuperGroupTestC5::TICKER, SuperGroupTestC1::TICKER].into_iter()
            )
        );
    }

    #[test]
    fn feed_pair() {
        fn build_price<QuoteG>() -> PriceDTO<SuperGroup, QuoteG>
        where
            SubGroupTestC1: MemberOf<QuoteG>,
            QuoteG: Group,
        {
            price::total_of(Coin::<SuperGroupTestC5>::new(1))
                .is(Coin::<SubGroupTestC1>::new(18500))
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
                &storage,
                NOW,
                TOTAL_FEEDERS,
                [SuperGroupTestC5::TICKER, SuperGroupTestC1::TICKER].into_iter()
            )
        );
        assert_eq!(
            Ok(build_price()),
            feeds.price::<SubGroupTestC1, SubGroup, _>(
                &storage,
                NOW,
                TOTAL_FEEDERS,
                [SuperGroupTestC5::TICKER, SubGroupTestC1::TICKER].into_iter()
            )
        );
    }

    #[test]
    fn feed_pairs() {
        let feeds = PriceFeeds::<SuperGroup>::new(FEEDS_NAMESPACE, config());
        let mut storage = MemoryStorage::new();
        let new_price75 =
            price::total_of(Coin::<SuperGroupTestC5>::new(1)).is(Coin::<SuperGroupTestC3>::new(2));
        let new_price56 =
            price::total_of(Coin::<SuperGroupTestC3>::new(1)).is(Coin::<SuperGroupTestC4>::new(3));
        let new_price51 =
            price::total_of(Coin::<SuperGroupTestC3>::new(1)).is(Coin::<SubGroupTestC1>::new(4));

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
                &storage,
                NOW,
                TOTAL_FEEDERS,
                [SuperGroupTestC5::TICKER, SuperGroupTestC2::TICKER].into_iter()
            )
        );
        assert_eq!(
            Ok(new_price75.into()),
            feeds.price::<SuperGroupTestC3, SuperGroup, _>(
                &storage,
                NOW,
                TOTAL_FEEDERS,
                [SuperGroupTestC5::TICKER, SuperGroupTestC3::TICKER].into_iter()
            )
        );
        assert_eq!(
            Ok(new_price56.into()),
            feeds.price::<SuperGroupTestC4, SuperGroup, _>(
                &storage,
                NOW,
                TOTAL_FEEDERS,
                [SuperGroupTestC3::TICKER, SuperGroupTestC4::TICKER].into_iter()
            )
        );
        assert_eq!(
            Ok(new_price51.into()),
            feeds.price::<SubGroupTestC1, SubGroup, _>(
                &storage,
                NOW,
                TOTAL_FEEDERS,
                [SuperGroupTestC3::TICKER, SubGroupTestC1::TICKER].into_iter()
            )
        );
        assert_eq!(
            Ok((new_price75 * new_price56).into()),
            feeds.price::<SuperGroupTestC4, SuperGroup, _>(
                &storage,
                NOW,
                TOTAL_FEEDERS,
                [
                    SuperGroupTestC5::TICKER,
                    SuperGroupTestC3::TICKER,
                    SuperGroupTestC4::TICKER
                ]
                .into_iter()
            )
        );
        assert_eq!(
            Ok((new_price75 * new_price51).into()),
            feeds.price::<SubGroupTestC1, SubGroup, _>(
                &storage,
                NOW,
                TOTAL_FEEDERS,
                [
                    SuperGroupTestC5::TICKER,
                    SuperGroupTestC3::TICKER,
                    SubGroupTestC1::TICKER
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
