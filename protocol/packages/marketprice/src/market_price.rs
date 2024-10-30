use std::{marker::PhantomData, mem};

use currency::{
    self, Currency, CurrencyDTO, CurrencyDef, Group, InPoolWith, MemberOf, PairsGroup,
    PairsVisitor, PairsVisitorResult, SymbolStatic,
};
use finance::price::{
    base::BasePrice,
    dto::{with_price, PriceDTO, WithPrice},
    Price,
};
use sdk::{
    cosmwasm_std::{Addr, Storage, Timestamp},
    cw_storage_plus::Map,
};

use crate::{
    config::Config,
    error::PriceFeedsError,
    feed::{ObservationsReadRepo, ObservationsRepo, PriceFeed},
};

pub struct PriceFeeds<'config, PriceG, ObservationsRepoImpl> {
    observations_repo: ObservationsRepoImpl,
    config: &'config Config,
    _g: PhantomData<PriceG>,
}

impl<'config, PriceG, ObservationsRepoImpl> PriceFeeds<'config, PriceG, ObservationsRepoImpl> {
    pub fn wipe_out_v2(store: &mut dyn Storage) {
        const NAMESPACE: &str = "market_price";
        Map::<(SymbolStatic, SymbolStatic), Vec<u8>>::new(NAMESPACE).clear(store);
    }

    pub const fn new(observations_repo: ObservationsRepoImpl, config: &'config Config) -> Self {
        Self {
            observations_repo,
            config,
            _g: PhantomData,
        }
    }
}

impl<'config, PriceG, ObservationsRepoImpl> PriceFeeds<'config, PriceG, ObservationsRepoImpl>
where
    PriceG: Group<TopG = PriceG>,
    ObservationsRepoImpl: ObservationsReadRepo<Group = PriceG>,
{
    pub fn price<'m, 'a, QuoteC, QuoteG, Iter>(
        &'m self,
        quote_c: CurrencyDTO<QuoteG>,
        at: Timestamp,
        total_feeders: usize,
        leaf_to_root: Iter,
    ) -> Result<BasePrice<PriceG, QuoteC, QuoteG>, PriceFeedsError>
    where
        'm: 'a,
        PriceG: Group<TopG = PriceG>,
        QuoteC: CurrencyDef + PairsGroup<CommonGroup = PriceG>,
        QuoteC::Group: MemberOf<QuoteG> + MemberOf<PriceG>,
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
            at,
            total_feeders,
            c_dto: &quote_in_price_group,
            root_dto: quote_c,
            price: Price::<QuoteC, QuoteC>::identity(),
        }
        .do_collect()
    }

    pub fn price_of_feed<C, QuoteC>(
        &self,
        amount_c: &CurrencyDTO<PriceG>,
        quote_c: &CurrencyDTO<PriceG>,
        at: Timestamp,
        total_feeders: usize,
    ) -> Result<Price<C, QuoteC>, PriceFeedsError>
    where
        C: Currency,
        C: MemberOf<PriceG>,
        QuoteC: Currency,
        QuoteC: MemberOf<PriceG>,
    {
        PriceFeed::with(
            self.observations_repo
                .observations_read::<C, QuoteC>(amount_c, quote_c),
        )
        .calc_price(self.config, at, total_feeders)
    }
}

impl<'config, PriceG, ObservationsRepoImpl> PriceFeeds<'config, PriceG, ObservationsRepoImpl>
where
    PriceG: Group<TopG = PriceG>,
    ObservationsRepoImpl: ObservationsRepo<Group = PriceG>,
{
    /// Feed new price observations
    ///
    /// The time `at` must always flow monotonically forward!
    pub fn feed(
        &mut self,
        at: Timestamp,
        sender_raw: Addr,
        prices: &[PriceDTO<PriceG>], // TODO pass by value to avoid the deref below
    ) -> Result<(), PriceFeedsError> {
        prices.iter().try_for_each(|price| {
            self.add_observation(
                sender_raw.clone(),
                at,
                *price,
                self.config.feed_valid_since(at),
            )
        })
    }

    fn add_observation(
        &mut self,
        from: Addr,
        at: Timestamp,
        price: PriceDTO<PriceG>,
        valid_since: Timestamp,
    ) -> Result<(), PriceFeedsError> {
        debug_assert!(valid_since < at);
        struct AddObservation<'feeds, G, ObservationsRepoImpl>
        where
            G: Group,
        {
            observations: &'feeds mut ObservationsRepoImpl,
            amount_c: CurrencyDTO<G>,
            quote_c: CurrencyDTO<G>,
            from: Addr,
            at: Timestamp,
            valid_since: Timestamp,
            group: PhantomData<G>,
        }

        impl<'feeds, G, ObservationsRepoImpl> WithPrice for AddObservation<'feeds, G, ObservationsRepoImpl>
        where
            G: Group<TopG = G>,
            ObservationsRepoImpl: ObservationsRepo<Group = G>,
        {
            type G = G;
            type Output = ();
            type Error = PriceFeedsError;

            fn exec<C, QuoteC>(self, price: Price<C, QuoteC>) -> Result<Self::Output, Self::Error>
            where
                C: Currency,
                QuoteC: Currency,
            {
                PriceFeed::with(
                    self.observations
                        .observations::<C, QuoteC>(&self.amount_c, &self.quote_c),
                )
                .add_observation(self.from, self.at, price, self.valid_since)
                .map(mem::drop)
            }
        }
        with_price::execute(
            price,
            AddObservation {
                observations: &mut self.observations_repo,
                amount_c: price.base().currency(),
                quote_c: price.quote().currency(),
                from,
                at,
                valid_since,
                group: PhantomData,
            },
        )
    }
}

struct PriceCollect<
    'a,
    'config,
    'currency,
    Iter,
    QuoteC,
    G,
    QuoteQuoteC,
    QuoteG,
    ObservationsRepoImpl,
> where
    Iter: Iterator<Item = &'a CurrencyDTO<G>>,
    QuoteC: Currency + MemberOf<G>,
    G: Group,
    QuoteQuoteC: Currency + MemberOf<QuoteG>,
    QuoteG: Group,
{
    root_to_leaf: Iter,
    feeds: &'a PriceFeeds<'config, G, ObservationsRepoImpl>,
    at: Timestamp,
    total_feeders: usize,
    c_dto: &'currency CurrencyDTO<G>,
    root_dto: CurrencyDTO<QuoteG>,
    price: Price<QuoteC, QuoteQuoteC>,
}
impl<'a, 'config, 'currency, Iter, C, G, QuoteC, QuoteG, ObservationsRepoImpl>
    PriceCollect<'a, 'config, 'currency, Iter, C, G, QuoteC, QuoteG, ObservationsRepoImpl>
where
    Iter: Iterator<Item = &'a CurrencyDTO<G>>,
    C: CurrencyDef + PairsGroup<CommonGroup = G>,
    C::Group: MemberOf<G>,
    G: 'a + Group<TopG = G>,
    QuoteC: CurrencyDef,
    QuoteC::Group: MemberOf<QuoteG> + MemberOf<G>,
    QuoteG: Group,
    ObservationsRepoImpl: ObservationsReadRepo<Group = G>,
{
    fn advance<'new_currency, NextC>(
        self,
        accumulator: Price<NextC, QuoteC>,
        c_dto: &'new_currency CurrencyDTO<G>,
    ) -> PriceCollect<
        'a,
        'config,
        'new_currency,
        Iter,
        NextC,
        G,
        QuoteC,
        QuoteG,
        ObservationsRepoImpl,
    >
    where
        'currency: 'new_currency,
        NextC: Currency + MemberOf<G> + PairsGroup<CommonGroup = G>,
    {
        PriceCollect {
            root_to_leaf: self.root_to_leaf,
            feeds: self.feeds,
            at: self.at,
            total_feeders: self.total_feeders,
            c_dto,
            root_dto: self.root_dto,
            price: accumulator,
        }
    }

    fn do_collect(mut self) -> Result<BasePrice<G, QuoteC, QuoteG>, PriceFeedsError> {
        if let Some(next_currency) = self.root_to_leaf.next() {
            next_currency.into_pair_member_type(self)
        } else {
            Ok(self.price.into())
        }
    }
}
impl<'a, 'config, 'currency, Iter, QuoteC, G, QuoteQuoteC, QuoteG, ObservationsRepoImpl>
    PairsVisitor
    for PriceCollect<
        'a,
        'config,
        'currency,
        Iter,
        QuoteC,
        G,
        QuoteQuoteC,
        QuoteG,
        ObservationsRepoImpl,
    >
where
    Iter: Iterator<Item = &'a CurrencyDTO<G>>,
    QuoteC: CurrencyDef + PairsGroup<CommonGroup = G>,
    QuoteC::Group: MemberOf<G>,
    G: Group<TopG = G>,
    QuoteQuoteC: CurrencyDef,
    QuoteQuoteC::Group: MemberOf<QuoteG> + MemberOf<G>,
    QuoteG: Group,
    ObservationsRepoImpl: ObservationsReadRepo<Group = G>,
{
    type Pivot = QuoteC;

    type Output = BasePrice<G, QuoteQuoteC, QuoteG>;
    type Error = PriceFeedsError;

    fn on<C>(self, def: &CurrencyDTO<C::Group>) -> PairsVisitorResult<Self>
    where
        C: CurrencyDef + InPoolWith<Self::Pivot> + PairsGroup<CommonGroup = G>,
        C::Group: MemberOf<G>,
    {
        let next_c = def.into_super_group::<G>();
        let next_price = self.feeds.price_of_feed::<C, QuoteC>(
            &next_c,
            self.c_dto,
            self.at,
            self.total_feeders,
        )?;
        let total_price = next_price * self.price;
        self.advance(total_price, &next_c).do_collect()
    }
}

#[cfg(test)]
mod test {
    use currency::test::{
        SubGroup, SubGroupTestC10, SuperGroup, SuperGroupTestC1, SuperGroupTestC2,
        SuperGroupTestC3, SuperGroupTestC4, SuperGroupTestC5,
    };
    use finance::{
        coin::Coin,
        duration::Duration,
        percent::Percent,
        price::{self, Price},
    };
    use sdk::cosmwasm_std::{testing::MockStorage, Addr, Storage, Timestamp};

    use crate::{error::PriceFeedsError, market_price::Config, Repo};

    use super::PriceFeeds;

    const FEEDER: &str = "0xifeege";
    const ROOT_NS: &str = "root_ns";
    const TOTAL_FEEDERS: usize = 1;
    const FEED_VALIDITY: Duration = Duration::from_secs(30);
    const SAMPLE_PERIOD_SECS: Duration = Duration::from_secs(5);
    const SAMPLES_NUMBER: u16 = 6;
    const DISCOUNTING_FACTOR: Percent = Percent::from_permille(750);

    const NOW: Timestamp = Timestamp::from_seconds(FEED_VALIDITY.secs() * 2);

    #[test]
    fn no_feed() {
        let config = config();
        let mut storage = MockStorage::new();
        let storage_dyn_ref: &mut dyn Storage = &mut storage;
        let feeds = PriceFeeds::new(Repo::new(ROOT_NS, storage_dyn_ref), &config);

        assert_eq!(
            Ok(Price::<SuperGroupTestC1, SuperGroupTestC1>::identity().into()),
            feeds.price::<SuperGroupTestC1, SuperGroup, _>(
                currency::dto::<SuperGroupTestC1, _>(),
                NOW,
                TOTAL_FEEDERS,
                [&currency::dto::<SuperGroupTestC1, _>(),].into_iter()
            )
        );

        assert_eq!(
            Err(PriceFeedsError::NoPrice()),
            feeds.price::<SuperGroupTestC1, SuperGroup, _>(
                currency::dto::<SuperGroupTestC1, _>(),
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
        fn build_price() -> Price<SuperGroupTestC5, SubGroupTestC10> {
            price::total_of(Coin::<SuperGroupTestC5>::new(1))
                .is(Coin::<SubGroupTestC10>::new(18500))
        }

        let config = config();
        let mut storage = MockStorage::new();
        let storage_dyn_ref: &mut dyn Storage = &mut storage;
        let mut feeds = PriceFeeds::new(Repo::new(ROOT_NS, storage_dyn_ref), &config);
        feeds
            .feed(NOW, Addr::unchecked(FEEDER), &[build_price().into()])
            .unwrap();

        assert_eq!(
            Err(PriceFeedsError::NoPrice()),
            feeds.price::<SuperGroupTestC1, SuperGroup, _>(
                currency::dto::<SuperGroupTestC1, _>(),
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
            Ok(build_price().into()),
            feeds.price::<SubGroupTestC10, SubGroup, _>(
                currency::dto::<SubGroupTestC10, _>(),
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
        let config = config();
        let mut storage = MockStorage::new();
        let storage_dyn_ref: &mut dyn Storage = &mut storage;
        let mut feeds = PriceFeeds::new(Repo::new(ROOT_NS, storage_dyn_ref), &config);
        let new_price75: Price<SuperGroupTestC5, SuperGroupTestC3> =
            price::total_of(Coin::new(1)).is(Coin::new(2));
        let new_price56 =
            price::total_of(Coin::<SuperGroupTestC3>::new(1)).is(Coin::<SuperGroupTestC4>::new(3));
        let new_price51 =
            price::total_of(Coin::<SuperGroupTestC3>::new(1)).is(Coin::<SubGroupTestC10>::new(4));

        feeds
            .feed(
                NOW,
                Addr::unchecked(FEEDER),
                &[new_price51.into(), new_price75.into(), new_price56.into()],
            )
            .unwrap();

        assert_eq!(
            Err(PriceFeedsError::NoPrice()),
            feeds.price::<SuperGroupTestC2, SuperGroup, _>(
                currency::dto::<SuperGroupTestC2, _>(),
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
