use std::str::FromStr;
use std::time::SystemTime;

use cosmwasm_std::testing::mock_dependencies;
use cosmwasm_std::{Api, Decimal256, Timestamp};

use crate::feeders::PriceFeeders;
use crate::market_price::{PriceFeeds, PriceFeedsError, PriceQuery};

#[test]
fn register_feeder() {
    let mut deps = mock_dependencies();

    let control = PriceFeeders::new("foo");
    let f_address = deps.api.addr_validate("address1").unwrap();
    control.register(deps.as_mut(), f_address.clone()).unwrap();

    let resp = control.is_registered(deps.as_ref(), &f_address).unwrap();
    assert!(resp);

    let feeders = control.get(deps.as_ref()).unwrap();
    assert_eq!(1, feeders.len());

    // should return error that address is already added
    let res = control.register(deps.as_mut(), f_address);
    assert!(res.is_ok());

    let f_address = deps.api.addr_validate("address2").unwrap();
    control.register(deps.as_mut(), f_address).unwrap();

    let f_address = deps.api.addr_validate("address3").unwrap();
    control.register(deps.as_mut(), f_address).unwrap();

    let feeders = control.get(deps.as_ref()).unwrap();
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
    let query = PriceQuery::new(("DEN1".to_string(), "DEN2".to_string()), 60, 50);
    let expected_err = market.get(&deps.storage, ts, query).unwrap_err();
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

    let prices: Vec<(String, Decimal256)> = Vec::new();
    market
        .feed(
            &mut deps.storage,
            ts,
            f_address,
            "DEN1".to_string(),
            prices,
            60,
        )
        .unwrap();
}

#[test]
fn marketprice_add_feed() {
    let mut deps = mock_dependencies();

    let market = PriceFeeds::new("foo");
    let f_address = deps.api.addr_validate("address1").unwrap();

    let prices: Vec<(String, Decimal256)> = vec![
        ("DEN2".to_string(), Decimal256::from_str("0.5").unwrap()),
        (
            "DEN3".to_string(),
            Decimal256::from_str("0.1000000009").unwrap(),
        ),
        (
            "DEN4".to_string(),
            Decimal256::from_str("1.00000000000002").unwrap(),
        ),
    ];

    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap();
    let ts = Timestamp::from_seconds(now.as_secs());

    market
        .feed(
            &mut deps.storage,
            ts,
            f_address,
            "DEN1".to_string(),
            prices,
            60,
        )
        .unwrap();
    let query = PriceQuery::new(("DEN1".to_string(), "DEN2".to_string()), 60, 50);
    let err = market.get(&deps.storage, ts, query).unwrap_err();
    assert_eq!(err, PriceFeedsError::NoPrice {});

    let query = PriceQuery::new(("DEN1".to_string(), "DEN2".to_string()), 60, 1);
    let price_resp = market.get(&deps.storage, ts, query).unwrap();
    assert_eq!(price_resp.price().to_string(), "0.5".to_string());
}
