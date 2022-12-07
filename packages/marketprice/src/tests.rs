use std::time::SystemTime;

use currency::{
    lease::{Atom, Cro, Evmos, Juno, Osmo, Wbtc, Weth},
    lpn::Usdc,
};
use finance::{
    coin::Coin,
    currency::Currency,
    duration::Duration,
    price::{self, dto::PriceDTO, Price},
};
use sdk::cosmwasm_std::{testing::mock_dependencies, Api, DepsMut, Timestamp};

use crate::{
    error::PriceFeedsError,
    feeders::PriceFeeders,
    market_price::{Config, PriceFeeds},
    SpotPrice,
};

const MINUTE: Duration = Duration::from_secs(60);

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
    let market = PriceFeeds::new("foo");

    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap();
    let ts = Timestamp::from_seconds(now.as_secs());
    let config = Config::new(MINUTE, 50, ts);
    let expected_err = market
        .price::<Atom, _>(
            &deps.storage,
            &config,
            [Osmo::TICKER, Atom::TICKER].into_iter(),
        )
        .unwrap_err();
    assert_eq!(expected_err, PriceFeedsError::NoPrice {});
}

#[test]
fn marketprice_add_feed_empty_vec() {
    let mut deps = mock_dependencies();

    let market = PriceFeeds::new("foo");
    let f_address = deps.api.addr_validate("address1").unwrap();

    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap();
    let ts = Timestamp::from_seconds(now.as_secs());

    let prices: Vec<SpotPrice> = Vec::new();
    market
        .feed(&mut deps.storage, ts, &f_address, &prices, MINUTE)
        .unwrap();
}

#[test]
fn marketprice_add_feed() {
    let mut deps = mock_dependencies();

    let market = PriceFeeds::new("foo");
    let f_address = deps.api.addr_validate("address1").unwrap();

    let price1 = price::total_of(Coin::<Osmo>::new(10)).is(Coin::<Atom>::new(5));
    let price2 = price::total_of(Coin::<Osmo>::new(10000000000)).is(Coin::<Weth>::new(1000000009));
    let price3 =
        price::total_of(Coin::<Osmo>::new(10000000000000)).is(Coin::<Wbtc>::new(100000000000002));

    let prices = vec![price1.into(), price2.into(), price3.into()];

    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap();
    let ts = Timestamp::from_seconds(now.as_secs());

    market
        .feed(&mut deps.storage, ts, &f_address, &prices, MINUTE)
        .unwrap();
    // require 50 feeders available => NoPrice
    let query = Config::new(MINUTE, 50, ts);
    let err = market
        .price::<Atom, _>(
            &deps.storage,
            &query,
            [Osmo::TICKER, Atom::TICKER].into_iter(),
        )
        .unwrap_err();
    assert_eq!(err, PriceFeedsError::NoPrice {});

    // require 1 feeders available => Price
    let query = Config::new(MINUTE, 1, ts);
    let price_resp = market
        .price::<Atom, _>(
            &deps.storage,
            &query,
            [Osmo::TICKER, Atom::TICKER].into_iter(),
        )
        .unwrap();

    assert_eq!(PriceDTO::try_from(price1).unwrap(), price_resp);
}

#[test]
fn marketprice_follow_the_path() {
    let mut deps = mock_dependencies();
    let market = PriceFeeds::new("foo");

    feed_price(
        deps.as_mut(),
        &market,
        price::total_of(Coin::<Atom>::new(1)).is(Coin::<Weth>::new(1)),
    )
    .unwrap();
    feed_price(
        deps.as_mut(),
        &market,
        price::total_of(Coin::<Cro>::new(1)).is(Coin::<Usdc>::new(3)),
    )
    .unwrap();

    feed_price(
        deps.as_mut(),
        &market,
        price::total_of(Coin::<Cro>::new(1)).is(Coin::<Wbtc>::new(3)),
    )
    .unwrap();

    feed_price(
        deps.as_mut(),
        &market,
        price::total_of(Coin::<Atom>::new(1)).is(Coin::<Osmo>::new(1)),
    )
    .unwrap();

    feed_price(
        deps.as_mut(),
        &market,
        price::total_of(Coin::<Cro>::new(1)).is(Coin::<Usdc>::new(3)),
    )
    .unwrap();
    feed_price(
        deps.as_mut(),
        &market,
        price::total_of(Coin::<Osmo>::new(1)).is(Coin::<Cro>::new(2)),
    )
    .unwrap();

    feed_price(
        deps.as_mut(),
        &market,
        price::total_of(Coin::<Cro>::new(1)).is(Coin::<Osmo>::new(3)),
    )
    .unwrap();

    feed_price(
        deps.as_mut(),
        &market,
        price::total_of(Coin::<Evmos>::new(1)).is(Coin::<Wbtc>::new(3)),
    )
    .unwrap();

    feed_price(
        deps.as_mut(),
        &market,
        price::total_of(Coin::<Usdc>::new(1)).is(Coin::<Atom>::new(3)),
    )
    .unwrap();

    let last_feed_time = feed_price(
        deps.as_mut(),
        &market,
        price::total_of(Coin::<Juno>::new(1)).is(Coin::<Usdc>::new(3)),
    )
    .unwrap();

    // valid search denom pair
    let query = Config::new(MINUTE, 1, last_feed_time);
    let price_resp = market
        .price::<Usdc, _>(
            &deps.storage,
            &query,
            [Atom::TICKER, Osmo::TICKER, Cro::TICKER, Usdc::TICKER].into_iter(),
        )
        .unwrap();
    let expected = price::total_of(Coin::<Atom>::new(1)).is(Coin::<Usdc>::new(6));
    let expected_dto = PriceDTO::from(expected);

    assert_eq!(expected_dto, price_resp);

    // first and second part of denom pair are the same
    let query = Config::new(MINUTE, 1, last_feed_time);
    let price_resp = market
        .price::<Usdc, _>(
            &deps.storage,
            &query,
            [Atom::TICKER, Usdc::TICKER].into_iter(),
        )
        .unwrap_err();
    assert_eq!(price_resp, PriceFeedsError::NoPrice());

    // second part of denome pair doesn't exists in the storage
    let query = Config::new(MINUTE, 1, last_feed_time);
    assert_eq!(
        market
            .price::<Usdc, _>(
                &deps.storage,
                &query,
                [Wbtc::TICKER, Usdc::TICKER].into_iter(),
            )
            .unwrap_err(),
        PriceFeedsError::NoPrice()
    );

    // first part of denome pair doesn't exists in the storage
    let query = Config::new(MINUTE, 1, last_feed_time);
    assert_eq!(
        market
            .price::<Osmo, _>(
                &deps.storage,
                &query,
                [Wbtc::TICKER, Osmo::TICKER].into_iter()
            )
            .unwrap_err(),
        PriceFeedsError::NoPrice {}
    );
}

fn feed_price<C1, C2>(
    deps: DepsMut,
    market: &PriceFeeds,
    price: Price<C1, C2>,
) -> Result<Timestamp, PriceFeedsError>
where
    C1: Currency,
    C2: Currency,
{
    let f_address = deps.api.addr_validate("address1").unwrap();

    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap();
    let ts = Timestamp::from_seconds(now.as_secs());

    let price = SpotPrice::try_from(price).unwrap();

    market.feed(deps.storage, ts, &f_address, &[price], MINUTE)?;
    Ok(ts)
}
