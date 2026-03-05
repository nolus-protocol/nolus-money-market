use std::time::SystemTime;

use currency::test::{
    SubGroup, SubGroupTestC6, SubGroupTestC10, SuperGroup, SuperGroupTestC1, SuperGroupTestC2,
    SuperGroupTestC3, SuperGroupTestC4, SuperGroupTestC5,
};
use currency::{CurrencyDTO, CurrencyDef, Group, MemberOf};
use finance::coin::Amount;
use finance::price::base::BasePrice;
use finance::{
    coin::Coin,
    duration::Duration,
    percent::Percent100,
    price::{self, Price, dto::PriceDTO},
};
use sdk::{
    cosmwasm_std::{Storage, Timestamp, testing::MockStorage},
    testing,
};

use crate::Repo;
use crate::feed::ObservationsRepo;
use crate::feeders::Count;
use crate::{
    config::Config, error::PriceFeedsError, feeders::PriceFeeders, market_price::PriceFeeds,
};

const ROOT_NS: &str = "root_ns";
const TOTAL_FEEDERS: Count = Count::new_test(1);
const TWICE_TOTAL_FEEDERS: Count = Count::new_test(2);
const SAMPLE_PERIOD_SECS: u32 = 5;
const SAMPLES_NUMBER: u16 = 12;
const DISCOUNTING_FACTOR: Percent100 = Percent100::from_permille(750);

#[track_caller]
pub(super) fn assert_price<
    'config,
    'currency_dto,
    ObservationsRepoImpl,
    C,
    PriceG,
    BaseC,
    BaseG,
    CurrenciesToBaseC,
>(
    expected: Price<C, BaseC>,
    feeds: &PriceFeeds<'config, PriceG, ObservationsRepoImpl>,
    at: Timestamp,
    total_feeders: Count,
    leaf_to_base: CurrenciesToBaseC,
) where
    ObservationsRepoImpl: ObservationsRepo<Group = PriceG>,
    C: CurrencyDef,
    C::Group: MemberOf<PriceG>,
    PriceG: Group<TopG = PriceG> + 'currency_dto,
    BaseC: CurrencyDef,
    BaseC::Group: MemberOf<BaseG> + MemberOf<PriceG>,
    BaseG: Group + MemberOf<PriceG>,
    CurrenciesToBaseC: Iterator<Item = &'currency_dto CurrencyDTO<PriceG>> + DoubleEndedIterator,
{
    assert_eq!(
        BasePrice::<PriceG, BaseC, BaseG>::from(expected),
        price::<ObservationsRepoImpl, PriceG, BaseC, BaseG, CurrenciesToBaseC>(
            feeds,
            at,
            total_feeders,
            leaf_to_base
        )
        .unwrap()
    );
}

#[track_caller]
pub(super) fn assert_no_price<
    'config,
    'currency_dto,
    ObservationsRepoImpl,
    PriceG,
    BaseC,
    BaseG,
    CurrenciesToBaseC,
>(
    feeds: &PriceFeeds<'config, PriceG, ObservationsRepoImpl>,
    at: Timestamp,
    total_feeders: Count,
    leaf_to_base: CurrenciesToBaseC,
) where
    ObservationsRepoImpl: ObservationsRepo<Group = PriceG>,
    PriceG: Group<TopG = PriceG> + 'currency_dto,
    BaseC: CurrencyDef,
    BaseC::Group: MemberOf<BaseG> + MemberOf<PriceG>,
    BaseG: Group + MemberOf<PriceG>,
    CurrenciesToBaseC: Iterator<Item = &'currency_dto CurrencyDTO<PriceG>> + DoubleEndedIterator,
{
    assert!(matches!(
        price::<ObservationsRepoImpl, PriceG, BaseC, BaseG, CurrenciesToBaseC>(
            feeds,
            at,
            total_feeders,
            leaf_to_base
        )
        .unwrap_err(),
        PriceFeedsError::NoPrice()
    ));
}

#[test]
fn register_feeder() {
    let mut storage = MockStorage::default();

    let control = PriceFeeders::new("foo");
    let f_address = testing::user("address1");
    let resp = control.is_registered(&storage, &f_address).unwrap();
    assert!(!resp);

    control.register(&mut storage, f_address.clone()).unwrap();

    let resp = control.is_registered(&storage, &f_address).unwrap();
    assert!(resp);

    let feeders = control.feeders(&storage).unwrap();
    assert_eq!(1, feeders.len());

    // should return error that address is already added
    let res = control.register(&mut storage, f_address);
    assert!(res.is_err());

    let f_address = testing::user("address2");
    control.register(&mut storage, f_address).unwrap();

    let f_address = testing::user("address3");
    control.register(&mut storage, f_address).unwrap();

    let feeders = control.feeders(&storage).unwrap();
    assert_eq!(3, feeders.len());
}

#[test]
fn marketprice_add_feed_expect_err() {
    let config = config();
    let mut storage = MockStorage::new();
    let storage_dyn_ref: &mut dyn Storage = &mut storage;
    let market = PriceFeeds::new(Repo::new(ROOT_NS, storage_dyn_ref), &config);

    assert_no_price::<_, _, SuperGroupTestC1, _, _>(
        &market,
        now(),
        TOTAL_FEEDERS,
        [
            &currency::dto::<SuperGroupTestC1, _>(),
            &currency::dto::<SuperGroupTestC2, _>(),
        ]
        .into_iter(),
    );
}

#[test]
fn marketprice_add_feed_empty_vec() {
    let config = config();
    let mut storage = MockStorage::new();
    let storage_dyn_ref: &mut dyn Storage = &mut storage;
    let mut market = PriceFeeds::new(Repo::new(ROOT_NS, storage_dyn_ref), &config);
    let f_address = testing::user("address1");

    let prices: Vec<PriceDTO<SuperGroup>> = Vec::new();
    market.feed(now(), f_address, &prices).unwrap();
}

#[test]
fn marketprice_add_feed() {
    let config = config();
    let mut storage = MockStorage::new();
    let storage_dyn_ref: &mut dyn Storage = &mut storage;
    let mut market = PriceFeeds::new(Repo::new(ROOT_NS, storage_dyn_ref), &config);
    let f_address = testing::user("address1");

    let price1 = price_new::<SuperGroupTestC1, SuperGroupTestC2>(10, 5);
    let price2 = price_new::<SuperGroupTestC1, SuperGroupTestC4>(10000000000000, 100000000000002);
    let price3 = price_new::<SuperGroupTestC1, SubGroupTestC10>(10000000000, 1000000009);

    let prices = vec![price1.into(), price2.into(), price3.into()];

    let now = now();
    market.feed(now, f_address, &prices).unwrap();
    assert_no_price::<_, _, SuperGroupTestC1, _, _>(
        &market,
        now,
        TWICE_TOTAL_FEEDERS,
        [
            &currency::dto::<SuperGroupTestC1, _>(),
            &currency::dto::<SuperGroupTestC4, _>(),
        ]
        .into_iter(),
    );

    assert_price(
        price2,
        &market,
        now,
        TOTAL_FEEDERS,
        [
            &currency::dto::<SuperGroupTestC1, _>(),
            &currency::dto::<SuperGroupTestC4, _>(),
        ]
        .into_iter(),
    );
}

#[test]
fn marketprice_follow_the_path() {
    let config = config();
    let mut storage = MockStorage::new();
    let storage_dyn_ref: &mut dyn Storage = &mut storage;
    let mut market = PriceFeeds::new(Repo::new(ROOT_NS, storage_dyn_ref), &config);

    feed_price(
        &mut market,
        price_new::<SuperGroupTestC1, SubGroupTestC10>(1, 4),
    )
    .unwrap();
    feed_price(
        &mut market,
        price_new::<SuperGroupTestC1, SuperGroupTestC2>(1, 3),
    )
    .unwrap();

    feed_price(
        &mut market,
        price_new::<SuperGroupTestC1, SubGroupTestC10>(1, 3),
    )
    .unwrap();

    feed_price(
        &mut market,
        price_new::<SuperGroupTestC5, SuperGroupTestC4>(5, 1),
    )
    .unwrap();

    feed_price(
        &mut market,
        price_new::<SuperGroupTestC1, SuperGroupTestC2>(1, 3),
    )
    .unwrap();
    feed_price(
        &mut market,
        price_new::<SuperGroupTestC4, SuperGroupTestC1>(1, 2),
    )
    .unwrap();

    feed_price(
        &mut market,
        price_new::<SubGroupTestC6, SubGroupTestC10>(1, 3),
    )
    .unwrap();

    feed_price(
        &mut market,
        price_new::<SuperGroupTestC2, SubGroupTestC10>(1, 3),
    )
    .unwrap();

    feed_price(
        &mut market,
        price_new::<SuperGroupTestC2, SuperGroupTestC3>(1, 3),
    )
    .unwrap();

    let last_feed_time = feed_price(
        &mut market,
        price_new::<SubGroupTestC6, SuperGroupTestC2>(1, 3),
    )
    .unwrap();

    assert_price(
        price_new::<SuperGroupTestC5, SuperGroupTestC2>(5, 6),
        &market,
        last_feed_time,
        TOTAL_FEEDERS,
        [
            &currency::dto::<SuperGroupTestC5, _>(),
            &currency::dto::<SuperGroupTestC4, _>(),
            &currency::dto::<SuperGroupTestC1, _>(),
            &currency::dto::<SuperGroupTestC2, _>(),
        ]
        .into_iter(),
    );

    // first and second part of denom pair are the same
    assert_no_price::<_, _, SuperGroupTestC3, _, _>(
        &market,
        last_feed_time,
        TOTAL_FEEDERS,
        [
            &currency::dto::<SuperGroupTestC3, _>(),
            &currency::dto::<SuperGroupTestC2, _>(),
        ]
        .into_iter(),
    );

    // second part of denome pair doesn't exists in the storage
    assert_no_price::<_, _, SubGroupTestC10, SubGroup, _>(
        &market,
        last_feed_time,
        TOTAL_FEEDERS,
        [
            &currency::dto::<SubGroupTestC10, _>(),
            &currency::dto::<SuperGroupTestC2, _>(),
        ]
        .into_iter(),
    );

    // first part of denome pair doesn't exists in the storage
    assert_no_price::<_, _, SubGroupTestC10, SubGroup, _>(
        &market,
        last_feed_time,
        TOTAL_FEEDERS,
        [
            &currency::dto::<SubGroupTestC10, _>(),
            &currency::dto::<SuperGroupTestC5, _>(),
        ]
        .into_iter(),
    );

    // the second leg doesn't exists in the storage
    assert_no_price::<_, _, SuperGroupTestC2, _, _>(
        &market,
        last_feed_time,
        TOTAL_FEEDERS,
        [
            &currency::dto::<SuperGroupTestC2, _>(),
            &currency::dto::<SubGroupTestC10, _>(),
            &currency::dto::<SuperGroupTestC1, _>(),
        ]
        .into_iter(),
    );
}

fn feed_price<C1, C2, G, ObservationsRepoT>(
    market: &mut PriceFeeds<'_, G, ObservationsRepoT>,
    price: Price<C1, C2>,
) -> Result<Timestamp, PriceFeedsError>
where
    C1: CurrencyDef,
    C1::Group: MemberOf<G>,
    C2: CurrencyDef,
    C2::Group: MemberOf<G>,
    G: Group<TopG = G>,
    ObservationsRepoT: ObservationsRepo<Group = G>,
{
    let f_address = testing::user("address1");

    let now = now();

    let price = PriceDTO::<G>::from(price);

    market.feed(now, f_address, &[price]).map(|()| now)
}

fn price<'config, 'currency_dto, ObservationsRepoImpl, PriceG, BaseC, BaseG, CurrenciesToBaseC>(
    feeds: &PriceFeeds<'config, PriceG, ObservationsRepoImpl>,
    at: Timestamp,
    total_feeders: Count,
    leaf_to_base: CurrenciesToBaseC,
) -> Result<BasePrice<PriceG, BaseC, BaseG>, PriceFeedsError>
where
    ObservationsRepoImpl: ObservationsRepo<Group = PriceG>,
    PriceG: Group<TopG = PriceG> + 'currency_dto,
    BaseC: CurrencyDef,
    BaseC::Group: MemberOf<BaseG> + MemberOf<PriceG>,
    BaseG: Group + MemberOf<PriceG>,
    CurrenciesToBaseC: Iterator<Item = &'currency_dto CurrencyDTO<PriceG>> + DoubleEndedIterator,
{
    feeds.price::<BaseC, BaseG, CurrenciesToBaseC>(at, total_feeders, leaf_to_base)
}

fn config() -> Config {
    Config::new(
        Percent100::MAX,
        Duration::from_secs(SAMPLE_PERIOD_SECS),
        SAMPLES_NUMBER,
        DISCOUNTING_FACTOR,
    )
}

fn price_new<C1, C2>(coin1: Amount, coin2: Amount) -> Price<C1, C2>
where
    C1: 'static,
    C2: 'static,
{
    price::total_of(Coin::<C1>::new(coin1)).is(Coin::<C2>::new(coin2))
}

fn now() -> Timestamp {
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap();
    Timestamp::from_seconds(now.as_secs())
}
