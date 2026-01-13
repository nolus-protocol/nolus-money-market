use std::{marker::PhantomData, mem};

use currency::{
    self, AnyVisitor, Currency, CurrencyDTO, CurrencyDef, Group, InPoolWith, MemberOf, PairsGroup,
    PairsVisitor,
};
use finance::price::{
    Price,
    base::BasePrice,
    dto::{PriceDTO, WithPrice, with_price},
};
use sdk::cosmwasm_std::{Addr, Timestamp};

use crate::{
    config::Config,
    error::PriceFeedsError,
    feed::{ObservationsReadRepo, ObservationsRepo, PriceFeed},
    feeders::Count,
};

pub struct PriceFeeds<'config, PriceG, ObservationsRepoImpl> {
    observations_repo: ObservationsRepoImpl,
    config: &'config Config,
    _g: PhantomData<PriceG>,
}

impl<'config, PriceG, ObservationsRepoImpl> PriceFeeds<'config, PriceG, ObservationsRepoImpl> {
    pub const fn new(observations_repo: ObservationsRepoImpl, config: &'config Config) -> Self {
        Self {
            observations_repo,
            config,
            _g: PhantomData,
        }
    }
}

impl<PriceG, ObservationsRepoImpl> PriceFeeds<'_, PriceG, ObservationsRepoImpl>
where
    PriceG: Group<TopG = PriceG>,
    ObservationsRepoImpl: ObservationsReadRepo<Group = PriceG>,
{
    pub fn price<'self_, 'currency_dto, BaseC, BaseG, CurrenciesToBaseC>(
        &'self_ self,
        at: Timestamp,
        total_feeders: Count,
        mut leaf_to_base: CurrenciesToBaseC,
    ) -> Result<BasePrice<PriceG, BaseC, BaseG>, PriceFeedsError>
    where
        PriceG: Group<TopG = PriceG> + 'currency_dto,
        BaseC: CurrencyDef,
        BaseC::Group: MemberOf<BaseG> + MemberOf<PriceG>,
        BaseG: Group + MemberOf<PriceG>,
        CurrenciesToBaseC:
            Iterator<Item = &'currency_dto CurrencyDTO<PriceG>> + DoubleEndedIterator,
    {
        struct CurrencyResolver<
            'config,
            'feeds,
            'currency_dto,
            G,
            BaseC,
            BaseG,
            CurrenciesToBaseC,
            ObservationsRepoImpl,
        >
        where
            G: Group,
        {
            feeds: &'feeds PriceFeeds<'config, G, ObservationsRepoImpl>,
            at: Timestamp,
            total_feeders: Count,
            leaf_to_base: CurrenciesToBaseC,
            _base_c: PhantomData<BaseC>,
            _base_g: PhantomData<BaseG>,
            _currency_dto: PhantomData<&'currency_dto CurrencyDTO<G>>,
        }
        impl<'currency_dto, G, BaseC, BaseG, CurrenciesToBaseC, ObservationsRepoImpl> AnyVisitor<G>
            for CurrencyResolver<
                '_,
                '_,
                'currency_dto,
                G,
                BaseC,
                BaseG,
                CurrenciesToBaseC,
                ObservationsRepoImpl,
            >
        where
            G: Group<TopG = G> + 'currency_dto,
            BaseC: CurrencyDef,
            BaseC::Group: MemberOf<BaseG> + MemberOf<G::TopG>,
            BaseG: Group,
            CurrenciesToBaseC: Iterator<Item = &'currency_dto CurrencyDTO<G>> + DoubleEndedIterator,
            ObservationsRepoImpl: ObservationsReadRepo<Group = G>,
        {
            type Outcome = Result<BasePrice<G, BaseC, BaseG>, PriceFeedsError>;

            fn on<C>(self, def: &CurrencyDTO<C::Group>) -> Self::Outcome
            where
                C: CurrencyDef + PairsGroup<CommonGroup = <G as Group>::TopG>,
                C::Group: MemberOf<G> + MemberOf<<G as Group>::TopG>,
            {
                let c_in_price_group = def.into_super_group::<G>();
                PriceCollect {
                    leaf_to_base: self.leaf_to_base,
                    feeds: self.feeds,
                    at: self.at,
                    total_feeders: self.total_feeders,
                    current_c: &c_in_price_group,
                    _base_c: PhantomData::<BaseC>,
                    _base_g: PhantomData::<BaseG>,
                    price: Price::<C, C>::identity(),
                }
                .do_collect()
            }
        }

        if let Some(c) = leaf_to_base.next() {
            c.into_currency_type(CurrencyResolver {
                feeds: self,
                at,
                total_feeders,
                leaf_to_base,
                _base_c: PhantomData,
                _base_g: PhantomData,
                _currency_dto: PhantomData,
            })
        } else {
            unreachable!("a non-empty chain of currencies to calculate price for the first one")
        }
    }

    pub fn price_of_feed<C, QuoteC>(
        &self,
        amount_c: &CurrencyDTO<PriceG>,
        quote_c: &CurrencyDTO<PriceG>,
        at: Timestamp,
        total_feeders: Count,
    ) -> Result<Price<C, QuoteC>, PriceFeedsError>
    where
        C: Currency + MemberOf<PriceG>,
        QuoteC: Currency + MemberOf<PriceG>,
    {
        PriceFeed::with(
            self.observations_repo
                .observations_read::<C, QuoteC>(amount_c, quote_c),
        )
        .calc_price(self.config, at, total_feeders)
    }
}

impl<PriceG, ObservationsRepoImpl> PriceFeeds<'_, PriceG, ObservationsRepoImpl>
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
        prices: &[PriceDTO<PriceG>],
    ) -> Result<(), PriceFeedsError> {
        prices.iter().try_for_each(|price| {
            self.add_observation(
                sender_raw.clone(),
                at,
                price,
                &self.config.feed_valid_since(at),
            )
        })
    }

    fn add_observation(
        &mut self,
        from: Addr,
        at: Timestamp,
        price: &PriceDTO<PriceG>,
        valid_since: &Timestamp,
    ) -> Result<(), PriceFeedsError> {
        debug_assert!(valid_since < &at);
        struct AddObservation<'feeds, 'since, G, ObservationsRepoImpl>
        where
            G: Group,
        {
            observations: &'feeds mut ObservationsRepoImpl,
            amount_c: CurrencyDTO<G>,
            quote_c: CurrencyDTO<G>,
            from: Addr,
            at: Timestamp,
            valid_since: &'since Timestamp,
            group: PhantomData<G>,
        }

        impl<G, ObservationsRepoImpl> WithPrice for AddObservation<'_, '_, G, ObservationsRepoImpl>
        where
            G: Group<TopG = G>,
            ObservationsRepoImpl: ObservationsRepo<Group = G>,
        {
            type G = G;
            type Outcome = Result<(), PriceFeedsError>;

            fn exec<C, QuoteC>(self, price: Price<C, QuoteC>) -> Self::Outcome
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
    'currency_dto,
    'feeds,
    'config,
    'currency,
    CurrenciesToBaseC,
    C,
    CurrentC,
    G,
    BaseC,
    BaseG,
    ObservationsRepoImpl,
> where
    CurrenciesToBaseC: Iterator<Item = &'currency_dto CurrencyDTO<G>>,
    C: Currency + MemberOf<G>,
    CurrentC: 'static,
    G: Group + 'currency_dto,
    BaseC: Currency,
{
    leaf_to_base: CurrenciesToBaseC,
    feeds: &'feeds PriceFeeds<'config, G, ObservationsRepoImpl>,
    at: Timestamp,
    total_feeders: Count,
    current_c: &'currency CurrencyDTO<G>,
    _base_c: PhantomData<BaseC>,
    _base_g: PhantomData<BaseG>,
    price: Price<C, CurrentC>,
}
impl<
    'currency_dto,
    'feeds,
    'config,
    CurrenciesToBaseC,
    C,
    CurrentC,
    G,
    BaseC,
    BaseG,
    ObservationsRepoImpl,
>
    PriceCollect<
        'currency_dto,
        'feeds,
        'config,
        '_,
        CurrenciesToBaseC,
        C,
        CurrentC,
        G,
        BaseC,
        BaseG,
        ObservationsRepoImpl,
    >
where
    CurrenciesToBaseC: Iterator<Item = &'currency_dto CurrencyDTO<G>>,
    C: CurrencyDef,
    C::Group: MemberOf<G>,
    CurrentC: Currency + MemberOf<G> + PairsGroup<CommonGroup = G>,
    G: Group<TopG = G> + 'currency_dto,
    BaseC: CurrencyDef,
    BaseC::Group: MemberOf<BaseG> + MemberOf<G::TopG>,
    BaseG: Group,
    ObservationsRepoImpl: ObservationsReadRepo<Group = G>,
{
    fn advance<'new_currency, QuoteC>(
        self,
        accumulator: Price<C, QuoteC>,
        quote_c_dto: &'new_currency CurrencyDTO<G>,
    ) -> PriceCollect<
        'currency_dto,
        'feeds,
        'config,
        'new_currency,
        CurrenciesToBaseC,
        C,
        QuoteC,
        G,
        BaseC,
        BaseG,
        ObservationsRepoImpl,
    >
    where
        QuoteC: Currency + MemberOf<G>,
    {
        PriceCollect {
            leaf_to_base: self.leaf_to_base,
            feeds: self.feeds,
            at: self.at,
            total_feeders: self.total_feeders,
            current_c: quote_c_dto,
            _base_c: self._base_c,
            _base_g: self._base_g,
            price: accumulator,
        }
    }

    fn do_collect(mut self) -> Result<BasePrice<G, BaseC, BaseG>, PriceFeedsError> {
        if let Some(next_currency) = self.leaf_to_base.next() {
            next_currency.into_pair_member_type(self)
        } else {
            Ok(self.do_collect_base())
        }
    }

    fn do_collect_base(self) -> BasePrice<G, BaseC, BaseG> {
        debug_assert_eq!(self.current_c, &currency::dto::<BaseC, BaseG>());

        (self.price * Price::<CurrentC, BaseC>::identity())
            .expect("multiplication by the price identity should not overflow")
            .into()
    }
}
impl<'currency_dto, CurrenciesToBaseC, C, CurrentC, G, BaseC, BaseG, ObservationsRepoImpl>
    PairsVisitor
    for PriceCollect<
        'currency_dto,
        '_,
        '_,
        '_,
        CurrenciesToBaseC,
        C,
        CurrentC,
        G,
        BaseC,
        BaseG,
        ObservationsRepoImpl,
    >
where
    CurrenciesToBaseC: Iterator<Item = &'currency_dto CurrencyDTO<G>>,
    C: CurrencyDef,
    C::Group: MemberOf<G>,
    CurrentC: Currency + MemberOf<G> + PairsGroup<CommonGroup = G>,
    G: Group<TopG = G> + 'currency_dto,
    BaseC: CurrencyDef,
    BaseC::Group: MemberOf<BaseG> + MemberOf<G>,
    BaseG: Group,
    ObservationsRepoImpl: ObservationsReadRepo<Group = G>,
{
    type Pivot = CurrentC;

    type Outcome = Result<BasePrice<G, BaseC, BaseG>, PriceFeedsError>;

    fn on<QuoteC>(self, def: &CurrencyDTO<QuoteC::Group>) -> Self::Outcome
    where
        QuoteC: CurrencyDef
            + InPoolWith<Self::Pivot>
            + PairsGroup<CommonGroup = <Self::Pivot as PairsGroup>::CommonGroup>,
        QuoteC::Group: MemberOf<G>,
    {
        let quote_c = def.into_super_group::<G>();
        let next_price = self.feeds.price_of_feed::<CurrentC, QuoteC>(
            self.current_c,
            &quote_c,
            self.at,
            self.total_feeders,
        )?;
        (self.price * next_price)
            .ok_or_else(|| PriceFeedsError::overflow_mul(self.price, next_price))
            .and_then(|total_price| self.advance(total_price, &quote_c).do_collect())
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
        percent::Percent100,
        price::{self, Price},
    };
    use sdk::cosmwasm_std::{Addr, Storage, Timestamp, testing::MockStorage};

    use crate::{Repo, error::PriceFeedsError, feeders::Count, market_price::Config};

    use super::PriceFeeds;

    const FEEDER: &str = "0xifeege";
    const ROOT_NS: &str = "root_ns";
    const TOTAL_FEEDERS: Count = Count::new_test(1);
    const FEED_VALIDITY: Duration = Duration::from_secs(30);
    const SAMPLE_PERIOD_SECS: Duration = Duration::from_secs(5);
    const SAMPLES_NUMBER: u16 = 6;
    const DISCOUNTING_FACTOR: Percent100 = Percent100::from_permille(750);

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
                NOW,
                TOTAL_FEEDERS,
                [&currency::dto::<SuperGroupTestC1, _>(),].into_iter()
            )
        );

        assert_eq!(
            Err(PriceFeedsError::NoPrice()),
            feeds.price::<SuperGroupTestC1, SuperGroup, _>(
                NOW,
                TOTAL_FEEDERS,
                [
                    &currency::dto::<SuperGroupTestC4, _>(),
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
            feeds.price::<SuperGroupTestC4, SuperGroup, _>(
                NOW,
                TOTAL_FEEDERS,
                [
                    &currency::dto::<SuperGroupTestC5, _>(),
                    &currency::dto::<SuperGroupTestC4, _>(),
                ]
                .into_iter()
            )
        );
        assert_eq!(
            Ok(build_price().into()),
            feeds.price::<SubGroupTestC10, SubGroup, _>(
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
        let new_price21: Price<SuperGroupTestC2, SuperGroupTestC1> =
            price::total_of(Coin::new(1)).is(Coin::new(2));
        let new_price14 =
            price::total_of(Coin::<SuperGroupTestC1>::new(1)).is(Coin::<SuperGroupTestC4>::new(3));
        let new_price110 =
            price::total_of(Coin::<SuperGroupTestC1>::new(1)).is(Coin::<SubGroupTestC10>::new(4));

        feeds
            .feed(
                NOW,
                Addr::unchecked(FEEDER),
                &[new_price110.into(), new_price21.into(), new_price14.into()],
            )
            .unwrap();

        assert_eq!(
            Err(PriceFeedsError::NoPrice()),
            feeds.price::<SuperGroupTestC3, SuperGroup, _>(
                NOW,
                TOTAL_FEEDERS,
                [
                    &currency::dto::<SuperGroupTestC2, _>(),
                    &currency::dto::<SuperGroupTestC3, _>(),
                ]
                .into_iter()
            )
        );
        assert_eq!(
            Ok(new_price21.into()),
            feeds.price::<SuperGroupTestC1, SuperGroup, _>(
                NOW,
                TOTAL_FEEDERS,
                [
                    &currency::dto::<SuperGroupTestC2, _>(),
                    &currency::dto::<SuperGroupTestC1, _>(),
                ]
                .into_iter()
            )
        );
        assert_eq!(
            Ok(new_price14.into()),
            feeds.price::<SuperGroupTestC4, SuperGroup, _>(
                NOW,
                TOTAL_FEEDERS,
                [
                    &currency::dto::<SuperGroupTestC1, _>(),
                    &currency::dto::<SuperGroupTestC4, _>(),
                ]
                .into_iter()
            )
        );
        assert_eq!(
            Ok(new_price110.into()),
            feeds.price::<SubGroupTestC10, SubGroup, _>(
                NOW,
                TOTAL_FEEDERS,
                [
                    &currency::dto::<SuperGroupTestC1, _>(),
                    &currency::dto::<SubGroupTestC10, _>(),
                ]
                .into_iter()
            )
        );
        assert_eq!(
            Ok((new_price21 * new_price14).unwrap().into()),
            feeds.price::<SuperGroupTestC4, SuperGroup, _>(
                NOW,
                TOTAL_FEEDERS,
                [
                    &currency::dto::<SuperGroupTestC2, _>(),
                    &currency::dto::<SuperGroupTestC1, _>(),
                    &currency::dto::<SuperGroupTestC4, _>(),
                ]
                .into_iter()
            )
        );
        assert_eq!(
            Ok((new_price21 * new_price110).unwrap().into()),
            feeds.price::<SubGroupTestC10, SubGroup, _>(
                NOW,
                TOTAL_FEEDERS,
                [
                    &currency::dto::<SuperGroupTestC2, _>(),
                    &currency::dto::<SuperGroupTestC1, _>(),
                    &currency::dto::<SubGroupTestC10, _>(),
                ]
                .into_iter()
            )
        );
    }

    fn config() -> Config {
        Config::new(
            Percent100::HUNDRED,
            SAMPLE_PERIOD_SECS,
            SAMPLES_NUMBER,
            DISCOUNTING_FACTOR,
        )
    }
}
