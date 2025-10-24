use std::time::SystemTime;

use currency::test::{
    SubGroupTestC6, SubGroupTestC10, SuperGroup, SuperGroupTestC1, SuperGroupTestC2,
    SuperGroupTestC3, SuperGroupTestC4, SuperGroupTestC5,
};
use currency::{CurrencyDef, Group, MemberOf};
use finance::coin::Amount;
use finance::price::base::BasePrice;
use finance::{
    coin::Coin,
    duration::Duration,
    percent::Percent100,
    price::{self, Price, dto::PriceDTO},
};
use sdk::{
    cosmwasm_std::{
        Storage, Timestamp,
        testing::{self as cosmwasm_test, MockStorage},
    },
    testing,
};

use crate::Repo;
use crate::feed::ObservationsRepo;
use crate::{
    config::Config, error::PriceFeedsError, feeders::PriceFeeders, market_price::PriceFeeds,
};

const ROOT_NS: &str = "root_ns";
const TOTAL_FEEDERS: usize = 1;
const SAMPLE_PERIOD_SECS: u32 = 5;
const SAMPLES_NUMBER: u16 = 12;
const DISCOUNTING_FACTOR: Percent100 = Percent100::from_permille(750);

#[test]
fn register_feeder() {
    let mut deps = cosmwasm_test::mock_dependencies();

    let control = PriceFeeders::new("foo");
    let f_address = testing::user("address1");
    let resp = control.is_registered(&deps.storage, &f_address).unwrap();
    assert!(!resp);

    control.register(deps.as_mut(), f_address.clone()).unwrap();

    let resp = control.is_registered(&deps.storage, &f_address).unwrap();
    assert!(resp);

    let feeders = control.feeders(&deps.storage).unwrap();
    assert_eq!(1, feeders.len());

    // should return error that address is already added
    let res = control.register(deps.as_mut(), f_address);
    assert!(res.is_err());

    let f_address = testing::user("address2");
    control.register(deps.as_mut(), f_address).unwrap();

    let f_address = testing::user("address3");
    control.register(deps.as_mut(), f_address).unwrap();

    let feeders = control.feeders(&deps.storage).unwrap();
    assert_eq!(3, feeders.len());
}

#[test]
fn marketprice_add_feed_expect_err() {
    let config = config();
    let mut storage = MockStorage::new();
    let storage_dyn_ref: &mut dyn Storage = &mut storage;
    let market = PriceFeeds::new(Repo::new(ROOT_NS, storage_dyn_ref), &config);

    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap();
    let ts = Timestamp::from_seconds(now.as_secs());
    let expected_err = market
        .price::<SuperGroupTestC2, SuperGroup, _>(
            ts,
            TOTAL_FEEDERS,
            [
                &currency::dto::<SuperGroupTestC1, _>(),
                &currency::dto::<SuperGroupTestC2, _>(),
            ]
            .into_iter(),
        )
        .unwrap_err();
    assert_eq!(expected_err, PriceFeedsError::NoPrice {});
}

#[test]
fn marketprice_add_feed_empty_vec() {
    let config = config();
    let mut storage = MockStorage::new();
    let storage_dyn_ref: &mut dyn Storage = &mut storage;
    let mut market = PriceFeeds::new(Repo::new(ROOT_NS, storage_dyn_ref), &config);
    let f_address = testing::user("address1");

    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap();
    let ts = Timestamp::from_seconds(now.as_secs());

    let prices: Vec<PriceDTO<SuperGroup>> = Vec::new();
    market.feed(ts, f_address, &prices).unwrap();
}

#[test]
fn marketprice_add_feed() {
    let config = config();
    let mut storage = MockStorage::new();
    let storage_dyn_ref: &mut dyn Storage = &mut storage;
    let mut market = PriceFeeds::new(Repo::new(ROOT_NS, storage_dyn_ref), &config);
    let f_address = testing::user("address1");

    let price1 = price::<SuperGroupTestC1, SuperGroupTestC2>(10, 5);
    let price2 = price::<SuperGroupTestC1, SuperGroupTestC4>(10000000000000, 100000000000002);
    let price3 = price::<SuperGroupTestC1, SubGroupTestC10>(10000000000, 1000000009);

    let prices = vec![price1.into(), price2.into(), price3.into()];

    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap();
    let ts = Timestamp::from_seconds(now.as_secs());

    market.feed(ts, f_address, &prices).unwrap();
    let err = market
        .price::<SuperGroupTestC4, SuperGroup, _>(
            ts,
            TOTAL_FEEDERS + TOTAL_FEEDERS,
            [
                &currency::dto::<SuperGroupTestC1, _>(),
                &currency::dto::<SuperGroupTestC4, _>(),
            ]
            .into_iter(),
        )
        .unwrap_err();
    assert_eq!(err, PriceFeedsError::NoPrice {});

    {
        let price_resp = market
            .price::<SuperGroupTestC4, SuperGroup, _>(
                ts,
                TOTAL_FEEDERS,
                [
                    &currency::dto::<SuperGroupTestC1, _>(),
                    &currency::dto::<SuperGroupTestC4, _>(),
                ]
                .into_iter(),
            )
            .unwrap();

        assert_eq!(BasePrice::from(price2), price_resp);
    }
}

#[test]
fn marketprice_follow_the_path() {
    let config = config();
    let mut storage = MockStorage::new();
    let storage_dyn_ref: &mut dyn Storage = &mut storage;
    let mut market = PriceFeeds::new(Repo::new(ROOT_NS, storage_dyn_ref), &config);

    feed_price(
        &mut market,
        price::<SuperGroupTestC1, SubGroupTestC10>(1, 4),
    )
    .unwrap();
    feed_price(
        &mut market,
        price::<SuperGroupTestC1, SuperGroupTestC2>(1, 3),
    )
    .unwrap();

    feed_price(
        &mut market,
        price::<SuperGroupTestC1, SubGroupTestC10>(1, 3),
    )
    .unwrap();

    feed_price(
        &mut market,
        price::<SuperGroupTestC5, SuperGroupTestC4>(5, 1),
    )
    .unwrap();

    feed_price(
        &mut market,
        price::<SuperGroupTestC1, SuperGroupTestC2>(1, 3),
    )
    .unwrap();
    feed_price(
        &mut market,
        price::<SuperGroupTestC4, SuperGroupTestC1>(1, 2),
    )
    .unwrap();

    feed_price(&mut market, price::<SubGroupTestC6, SubGroupTestC10>(1, 3)).unwrap();

    feed_price(
        &mut market,
        price::<SuperGroupTestC2, SubGroupTestC10>(1, 3),
    )
    .unwrap();

    feed_price(
        &mut market,
        price::<SuperGroupTestC2, SuperGroupTestC3>(1, 3),
    )
    .unwrap();

    let last_feed_time =
        feed_price(&mut market, price::<SubGroupTestC6, SuperGroupTestC2>(1, 3)).unwrap();

    let price_resp = market
        .price::<SuperGroupTestC2, SuperGroup, _>(
            last_feed_time,
            TOTAL_FEEDERS,
            [
                &currency::dto::<SuperGroupTestC5, _>(),
                &currency::dto::<SuperGroupTestC4, _>(),
                &currency::dto::<SuperGroupTestC1, _>(),
                &currency::dto::<SuperGroupTestC2, _>(),
            ]
            .into_iter(),
        )
        .unwrap();
    let expected = price::<SuperGroupTestC5, SuperGroupTestC2>(5, 6);
    let expected_dto = BasePrice::from(expected);

    assert_eq!(expected_dto, price_resp);

    // first and second part of denom pair are the same
    let price_resp = market
        .price::<SuperGroupTestC2, SuperGroup, _>(
            last_feed_time,
            TOTAL_FEEDERS,
            [
                &currency::dto::<SuperGroupTestC3, _>(),
                &currency::dto::<SuperGroupTestC2, _>(),
            ]
            .into_iter(),
        )
        .unwrap_err();
    assert_eq!(price_resp, PriceFeedsError::NoPrice());

    // second part of denome pair doesn't exists in the storage
    assert_eq!(
        market
            .price::<SuperGroupTestC2, SuperGroup, _>(
                last_feed_time,
                TOTAL_FEEDERS,
                [
                    &currency::dto::<SubGroupTestC10, _>(),
                    &currency::dto::<SuperGroupTestC2, _>(),
                ]
                .into_iter(),
            )
            .unwrap_err(),
        PriceFeedsError::NoPrice()
    );

    // first part of denome pair doesn't exists in the storage
    assert_eq!(
        market
            .price::<SuperGroupTestC5, SuperGroup, _>(
                last_feed_time,
                TOTAL_FEEDERS,
                [
                    &currency::dto::<SubGroupTestC10, _>(),
                    &currency::dto::<SuperGroupTestC5, _>(),
                ]
                .into_iter()
            )
            .unwrap_err(),
        PriceFeedsError::NoPrice {}
    );

    // the second leg doesn't exists in the storage
    assert_eq!(
        market
            .price::<SuperGroupTestC2, SuperGroup, _>(
                last_feed_time,
                TOTAL_FEEDERS,
                [
                    &currency::dto::<SuperGroupTestC2, _>(),
                    &currency::dto::<SubGroupTestC10, _>(),
                    &currency::dto::<SuperGroupTestC1, _>(),
                ]
                .into_iter()
            )
            .unwrap_err(),
        PriceFeedsError::NoPrice {}
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

    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap();
    let ts = Timestamp::from_seconds(now.as_secs());

    let price = PriceDTO::<G>::from(price);

    market.feed(ts, f_address, &[price]).map(|()| ts)
}

fn config() -> Config {
    Config::new(
        Percent100::HUNDRED,
        Duration::from_secs(SAMPLE_PERIOD_SECS),
        SAMPLES_NUMBER,
        DISCOUNTING_FACTOR,
    )
}

fn price<C1, C2>(coin1: Amount, coin2: Amount) -> Price<C1, C2>
where
    C1: 'static,
    C2: 'static,
{
    price::total_of(Coin::<C1>::new(coin1)).is(Coin::<C2>::new(coin2))
}
