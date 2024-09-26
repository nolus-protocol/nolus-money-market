use std::time::SystemTime;

use currency::test::{
    SubGroupTestC10, SubGroupTestC6, SuperGroup, SuperGroupTestC1, SuperGroupTestC2,
    SuperGroupTestC3, SuperGroupTestC4, SuperGroupTestC5,
};
use currency::{CurrencyDef, Group, MemberOf};
use finance::price::base::BasePrice;
use finance::{
    coin::Coin,
    duration::Duration,
    percent::Percent,
    price::{self, dto::PriceDTO, Price},
};
use sdk::cosmwasm_std::{testing::mock_dependencies, Api, DepsMut, Timestamp};

use crate::{
    config::Config, error::PriceFeedsError, feeders::PriceFeeders, market_price::PriceFeeds,
};

const TOTAL_FEEDERS: usize = 1;
const SAMPLE_PERIOD_SECS: u32 = 5;
const SAMPLES_NUMBER: u16 = 12;
const DISCOUNTING_FACTOR: Percent = Percent::from_permille(750);

#[test]
fn register_feeder() {
    let mut deps = mock_dependencies();

    let control = PriceFeeders::new("foo");
    let f_address = deps.api.addr_validate("address1").unwrap();
    let resp = control.is_registered(&deps.storage, &f_address).unwrap();
    assert!(!resp);

    control.register(deps.as_mut(), f_address.clone()).unwrap();

    let resp = control.is_registered(&deps.storage, &f_address).unwrap();
    assert!(resp);

    let feeders = control.get(&deps.storage).unwrap();
    assert_eq!(1, feeders.len());

    // should return error that address is already added
    let res = control.register(deps.as_mut(), f_address);
    assert!(res.is_err());

    let f_address = deps.api.addr_validate("address2").unwrap();
    control.register(deps.as_mut(), f_address).unwrap();

    let f_address = deps.api.addr_validate("address3").unwrap();
    control.register(deps.as_mut(), f_address).unwrap();

    let feeders = control.get(&deps.storage).unwrap();
    assert_eq!(3, feeders.len());
}

#[test]
fn marketprice_add_feed_expect_err() {
    let deps = mock_dependencies();
    let market: PriceFeeds<SuperGroup> = PriceFeeds::new("foo", config());

    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap();
    let ts = Timestamp::from_seconds(now.as_secs());
    let expected_err = market
        .price::<SuperGroupTestC3, SuperGroup, _>(
            currency::dto::<SuperGroupTestC3, _>(),
            &deps.storage,
            ts,
            TOTAL_FEEDERS,
            [
                &currency::dto::<SuperGroupTestC5, _>(),
                &currency::dto::<SuperGroupTestC3, _>(),
            ]
            .into_iter(),
        )
        .unwrap_err();
    assert_eq!(expected_err, PriceFeedsError::NoPrice {});
}

#[test]
fn marketprice_add_feed_empty_vec() {
    let mut deps = mock_dependencies();
    let market = PriceFeeds::new("foo", config());
    let f_address = deps.api.addr_validate("address1").unwrap();

    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap();
    let ts = Timestamp::from_seconds(now.as_secs());

    let prices: Vec<PriceDTO<SuperGroup>> = Vec::new();
    market
        .feed(&mut deps.storage, ts, &f_address, &prices)
        .unwrap();
}

#[test]
fn marketprice_add_feed() {
    let mut deps = mock_dependencies();
    let market: PriceFeeds<SuperGroup> = PriceFeeds::new("foo", config());
    let f_address = deps.api.addr_validate("address1").unwrap();

    let price1 = price::<SuperGroupTestC5, SuperGroupTestC3, _, _>(10, 5);
    let price2 = price::<SuperGroupTestC5, SubGroupTestC10, _, _>(10000000000, 1000000009);
    let price3 = price::<SuperGroupTestC5, SuperGroupTestC4, _, _>(10000000000000, 100000000000002);

    let prices = vec![price1.into(), price2.into(), price3.into()];

    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap();
    let ts = Timestamp::from_seconds(now.as_secs());

    market
        .feed(&mut deps.storage, ts, &f_address, &prices)
        .unwrap();
    let err = market
        .price::<SuperGroupTestC3, SuperGroup, _>(
            currency::dto::<SuperGroupTestC3, _>(),
            &deps.storage,
            ts,
            TOTAL_FEEDERS + TOTAL_FEEDERS,
            [
                &currency::dto::<SuperGroupTestC5, _>(),
                &currency::dto::<SuperGroupTestC3, _>(),
            ]
            .into_iter(),
        )
        .unwrap_err();
    assert_eq!(err, PriceFeedsError::NoPrice {});

    {
        let price_resp = market
            .price::<SuperGroupTestC3, SuperGroup, _>(
                currency::dto::<SuperGroupTestC3, _>(),
                &deps.storage,
                ts,
                TOTAL_FEEDERS,
                [
                    &currency::dto::<SuperGroupTestC5, _>(),
                    &currency::dto::<SuperGroupTestC3, _>(),
                ]
                .into_iter(),
            )
            .unwrap();

        assert_eq!(BasePrice::from(price1), price_resp);
    }
}

#[test]
fn marketprice_follow_the_path() {
    let mut deps = mock_dependencies();
    let market: PriceFeeds<SuperGroup> = PriceFeeds::new("foo", config());

    feed_price(
        deps.as_mut(),
        &market,
        price::<SuperGroupTestC3, SubGroupTestC10, _, _>(1, 1),
    )
    .unwrap();
    feed_price(
        deps.as_mut(),
        &market,
        price::<SuperGroupTestC1, SuperGroupTestC2, _, _>(1, 3),
    )
    .unwrap();

    feed_price(
        deps.as_mut(),
        &market,
        price::<SuperGroupTestC1, SubGroupTestC10, _, _>(1, 3),
    )
    .unwrap();

    feed_price(
        deps.as_mut(),
        &market,
        price::<SuperGroupTestC3, SuperGroupTestC5, _, _>(1, 1),
    )
    .unwrap();

    feed_price(
        deps.as_mut(),
        &market,
        price::<SuperGroupTestC1, SuperGroupTestC2, _, _>(1, 3),
    )
    .unwrap();
    feed_price(
        deps.as_mut(),
        &market,
        price::<SuperGroupTestC5, SuperGroupTestC1, _, _>(1, 2),
    )
    .unwrap();

    feed_price(
        deps.as_mut(),
        &market,
        price::<SuperGroupTestC1, SuperGroupTestC5, _, _>(1, 3),
    )
    .unwrap();

    feed_price(
        deps.as_mut(),
        &market,
        price::<SuperGroupTestC4, SubGroupTestC10, _, _>(1, 3),
    )
    .unwrap();

    feed_price(
        deps.as_mut(),
        &market,
        price::<SuperGroupTestC2, SuperGroupTestC3, _, _>(1, 3),
    )
    .unwrap();

    let last_feed_time = feed_price(
        deps.as_mut(),
        &market,
        price::<SubGroupTestC6, SuperGroupTestC2, _, _>(1, 3),
    )
    .unwrap();

    let price_resp = market
        .price::<SuperGroupTestC2, SuperGroup, _>(
            currency::dto::<SuperGroupTestC2, _>(),
            &deps.storage,
            last_feed_time,
            TOTAL_FEEDERS,
            [
                &currency::dto::<SuperGroupTestC3, _>(),
                &currency::dto::<SuperGroupTestC5, _>(),
                &currency::dto::<SuperGroupTestC1, _>(),
                &currency::dto::<SuperGroupTestC2, _>(),
            ]
            .into_iter(),
        )
        .unwrap();
    let expected = price::<SuperGroupTestC3, SuperGroupTestC2, _, _>(1, 6);
    let expected_dto = BasePrice::from(expected);

    assert_eq!(expected_dto, price_resp);

    // first and second part of denom pair are the same
    let price_resp = market
        .price::<SuperGroupTestC2, SuperGroup, _>(
            currency::dto::<SuperGroupTestC2, _>(),
            &deps.storage,
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
                currency::dto::<SuperGroupTestC2, _>(),
                &deps.storage,
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
                currency::dto::<SuperGroupTestC5, _>(),
                &deps.storage,
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
                currency::dto::<SuperGroupTestC2, _>(),
                &deps.storage,
                last_feed_time,
                TOTAL_FEEDERS,
                [
                    &currency::dto::<SuperGroupTestC3, _>(),
                    &currency::dto::<SubGroupTestC10, _>(),
                    &currency::dto::<SuperGroupTestC2, _>(),
                ]
                .into_iter()
            )
            .unwrap_err(),
        PriceFeedsError::NoPrice {}
    );
}

fn feed_price<C1, C2, G>(
    deps: DepsMut<'_>,
    market: &PriceFeeds<G>,
    price: Price<C1, C2>,
) -> Result<Timestamp, PriceFeedsError>
where
    C1: CurrencyDef,
    C1::Group: MemberOf<G>,
    C2: CurrencyDef,
    C2::Group: MemberOf<G>,
    G: Group<TopG = G>,
{
    let f_address = deps.api.addr_validate("address1").unwrap();

    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap();
    let ts = Timestamp::from_seconds(now.as_secs());

    let price = PriceDTO::<G>::from(price);

    market.feed(deps.storage, ts, &f_address, &[price])?;
    Ok(ts)
}

fn config() -> Config {
    Config::new(
        Percent::HUNDRED,
        Duration::from_secs(SAMPLE_PERIOD_SECS),
        SAMPLES_NUMBER,
        DISCOUNTING_FACTOR,
    )
}

fn price<C1, C2, CoinC1, CoinC2>(coin1: CoinC1, coin2: CoinC2) -> Price<C1, C2>
where
    C1: 'static,
    CoinC1: Into<Coin<C1>>,
    C2: 'static,
    CoinC2: Into<Coin<C2>>,
{
    price::total_of(coin1.into()).is(coin2.into())
}
