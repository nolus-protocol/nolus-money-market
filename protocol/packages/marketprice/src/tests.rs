use std::time::SystemTime;

use currency::test::{
    SubGroupTestC1, SuperGroup, SuperGroupTestC1, SuperGroupTestC2, SuperGroupTestC3,
    SuperGroupTestC4, SuperGroupTestC5, SuperGroupTestC6,
};
use currency::{Currency, Group};
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
    let market: PriceFeeds<'_, SuperGroup> = PriceFeeds::new("foo", config());

    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap();
    let ts = Timestamp::from_seconds(now.as_secs());
    let expected_err = market
        .price::<SuperGroupTestC3, SuperGroup, _>(
            &deps.storage,
            ts,
            TOTAL_FEEDERS,
            [SuperGroupTestC5::TICKER, SuperGroupTestC3::TICKER].into_iter(),
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

    let prices: Vec<PriceDTO<SuperGroup, SuperGroup>> = Vec::new();
    market
        .feed(&mut deps.storage, ts, &f_address, &prices)
        .unwrap();
}

#[test]
fn marketprice_add_feed() {
    let mut deps = mock_dependencies();
    let market: PriceFeeds<'_, SuperGroup> = PriceFeeds::new("foo", config());
    let f_address = deps.api.addr_validate("address1").unwrap();

    let price1 =
        price::total_of(Coin::<SuperGroupTestC5>::new(10)).is(Coin::<SuperGroupTestC3>::new(5));
    let price2 =
        price::total_of(Coin::<SuperGroupTestC5>::new(10000000000))
            .is(Coin::<SubGroupTestC1>::new(1000000009));
    let price3 =
        price::total_of(Coin::<SuperGroupTestC5>::new(10000000000000))
            .is(Coin::<SuperGroupTestC4>::new(100000000000002));

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
            &deps.storage,
            ts,
            TOTAL_FEEDERS + TOTAL_FEEDERS,
            [SuperGroupTestC5::TICKER, SuperGroupTestC3::TICKER].into_iter(),
        )
        .unwrap_err();
    assert_eq!(err, PriceFeedsError::NoPrice {});

    {
        let price_resp = market
            .price::<SuperGroupTestC3, SuperGroup, _>(
                &deps.storage,
                ts,
                TOTAL_FEEDERS,
                [SuperGroupTestC5::TICKER, SuperGroupTestC3::TICKER].into_iter(),
            )
            .unwrap();

        assert_eq!(PriceDTO::try_from(price1).unwrap(), price_resp);
    }
}

#[test]
fn marketprice_follow_the_path() {
    let mut deps = mock_dependencies();
    let market: PriceFeeds<'_, SuperGroup> = PriceFeeds::new("foo", config());

    feed_price(
        deps.as_mut(),
        &market,
        price::total_of(Coin::<SuperGroupTestC3>::new(1)).is(Coin::<SubGroupTestC1>::new(1)),
    )
    .unwrap();
    feed_price(
        deps.as_mut(),
        &market,
        price::total_of(Coin::<SuperGroupTestC1>::new(1)).is(Coin::<SuperGroupTestC2>::new(3)),
    )
    .unwrap();

    feed_price(
        deps.as_mut(),
        &market,
        price::total_of(Coin::<SuperGroupTestC1>::new(1)).is(Coin::<SubGroupTestC1>::new(3)),
    )
    .unwrap();

    feed_price(
        deps.as_mut(),
        &market,
        price::total_of(Coin::<SuperGroupTestC3>::new(1)).is(Coin::<SuperGroupTestC5>::new(1)),
    )
    .unwrap();

    feed_price(
        deps.as_mut(),
        &market,
        price::total_of(Coin::<SuperGroupTestC1>::new(1)).is(Coin::<SuperGroupTestC2>::new(3)),
    )
    .unwrap();
    feed_price(
        deps.as_mut(),
        &market,
        price::total_of(Coin::<SuperGroupTestC5>::new(1)).is(Coin::<SuperGroupTestC1>::new(2)),
    )
    .unwrap();

    feed_price(
        deps.as_mut(),
        &market,
        price::total_of(Coin::<SuperGroupTestC1>::new(1)).is(Coin::<SuperGroupTestC5>::new(3)),
    )
    .unwrap();

    feed_price(
        deps.as_mut(),
        &market,
        price::total_of(Coin::<SuperGroupTestC4>::new(1)).is(Coin::<SubGroupTestC1>::new(3)),
    )
    .unwrap();

    feed_price(
        deps.as_mut(),
        &market,
        price::total_of(Coin::<SuperGroupTestC2>::new(1)).is(Coin::<SuperGroupTestC3>::new(3)),
    )
    .unwrap();

    let last_feed_time = feed_price(
        deps.as_mut(),
        &market,
        price::total_of(Coin::<SuperGroupTestC6>::new(1)).is(Coin::<SuperGroupTestC2>::new(3)),
    )
    .unwrap();

    let price_resp = market
        .price::<SuperGroupTestC2, SuperGroup, _>(
            &deps.storage,
            last_feed_time,
            TOTAL_FEEDERS,
            [
                SuperGroupTestC3::TICKER,
                SuperGroupTestC5::TICKER,
                SuperGroupTestC1::TICKER,
                SuperGroupTestC2::TICKER,
            ]
            .into_iter(),
        )
        .unwrap();
    let expected =
        price::total_of(Coin::<SuperGroupTestC3>::new(1)).is(Coin::<SuperGroupTestC2>::new(6));
    let expected_dto = PriceDTO::from(expected);

    assert_eq!(expected_dto, price_resp);

    // first and second part of denom pair are the same
    let price_resp = market
        .price::<SuperGroupTestC2, SuperGroup, _>(
            &deps.storage,
            last_feed_time,
            TOTAL_FEEDERS,
            [SuperGroupTestC3::TICKER, SuperGroupTestC2::TICKER].into_iter(),
        )
        .unwrap_err();
    assert_eq!(price_resp, PriceFeedsError::NoPrice());

    // second part of denome pair doesn't exists in the storage
    assert_eq!(
        market
            .price::<SuperGroupTestC2, SuperGroup, _>(
                &deps.storage,
                last_feed_time,
                TOTAL_FEEDERS,
                [SubGroupTestC1::TICKER, SuperGroupTestC2::TICKER].into_iter(),
            )
            .unwrap_err(),
        PriceFeedsError::NoPrice()
    );

    // first part of denome pair doesn't exists in the storage
    assert_eq!(
        market
            .price::<SuperGroupTestC5, SuperGroup, _>(
                &deps.storage,
                last_feed_time,
                TOTAL_FEEDERS,
                [SubGroupTestC1::TICKER, SuperGroupTestC5::TICKER].into_iter()
            )
            .unwrap_err(),
        PriceFeedsError::NoPrice {}
    );
}

fn feed_price<C1, C2, G>(
    deps: DepsMut<'_>,
    market: &PriceFeeds<'_, G>,
    price: Price<C1, C2>,
) -> Result<Timestamp, PriceFeedsError>
where
    C1: Currency,
    C2: Currency,
    G: Group,
{
    let f_address = deps.api.addr_validate("address1").unwrap();

    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap();
    let ts = Timestamp::from_seconds(now.as_secs());

    let price = PriceDTO::<G, G>::try_from(price).unwrap();

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
