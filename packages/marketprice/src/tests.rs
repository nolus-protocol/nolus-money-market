use std::convert::TryFrom;
use std::time::SystemTime;

use cosmwasm_std::testing::{mock_dependencies, mock_env};
use cosmwasm_std::{Api, DepsMut, Timestamp};
use finance::coin::Coin;
use finance::currency::{
    Currency, SymbolStatic, TestCurrencyA, TestCurrencyB, TestCurrencyC, TestCurrencyD,
};
use finance::price::{self, dto::PriceDTO, Price};

use crate::error::PriceFeedsError;
use crate::feeders::PriceFeeders;
use crate::market_price::{Parameters, PriceFeeds};
use finance::duration::Duration;

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
    assert!(res.is_ok());

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
    let params = Parameters::new(MINUTE, 50, ts);
    let expected_err = market
        .price(
            &deps.storage,
            params,
            TestCurrencyA::SYMBOL.to_string(),
            TestCurrencyB::SYMBOL.to_string(),
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

    let prices: Vec<PriceDTO> = Vec::new();
    market
        .feed(&mut deps.storage, ts, &f_address, prices, MINUTE)
        .unwrap();
}

#[test]
fn marketprice_add_feed() {
    let mut deps = mock_dependencies();

    let market = PriceFeeds::new("foo");
    let f_address = deps.api.addr_validate("address1").unwrap();

    let price1 = price::total_of(Coin::<TestCurrencyA>::new(10)).is(Coin::<TestCurrencyB>::new(5));
    let price2 = price::total_of(Coin::<TestCurrencyA>::new(10000000000))
        .is(Coin::<TestCurrencyC>::new(1000000009));
    let price3 =
        price::total_of(Coin::<TestCurrencyA>::new(10000000000000))
            .is(Coin::<TestCurrencyD>::new(100000000000002));

    let prices: Vec<PriceDTO> = vec![
        PriceDTO::try_from(price1).unwrap(),
        PriceDTO::try_from(price2).unwrap(),
        PriceDTO::try_from(price3).unwrap(),
    ];

    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap();
    let ts = Timestamp::from_seconds(now.as_secs());

    market
        .feed(&mut deps.storage, ts, &f_address, prices, MINUTE)
        .unwrap();
    let query = Parameters::new(MINUTE, 50, ts);
    let err = market
        .price(
            &deps.storage,
            query,
            TestCurrencyA::SYMBOL.to_string(),
            TestCurrencyB::SYMBOL.to_string(),
        )
        .unwrap_err();
    assert_eq!(err, PriceFeedsError::NoPrice {});

    let query = Parameters::new(MINUTE, 1, ts);
    let price_resp = market
        .price(
            &deps.storage,
            query,
            TestCurrencyA::SYMBOL.to_string(),
            TestCurrencyB::SYMBOL.to_string(),
        )
        .unwrap();

    assert_eq!(PriceDTO::try_from(price1).unwrap(), price_resp);
}

#[test]
fn marketprice_follow_the_path() {
    #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default)]
    pub struct DEN0;
    impl Currency for DEN0 {
        const SYMBOL: SymbolStatic = "DEN0";
    }
    #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default)]
    pub struct DENX;
    impl Currency for DENX {
        const SYMBOL: SymbolStatic = "DENX";
    }
    #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default)]
    pub struct DENZ;
    impl Currency for DENZ {
        const SYMBOL: SymbolStatic = "DENZ";
    }
    #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default)]
    pub struct DENC;
    impl Currency for DENC {
        const SYMBOL: SymbolStatic = "DENC";
    }

    let mut deps = mock_dependencies();
    let market = PriceFeeds::new("foo");

    feed_price(
        deps.as_mut(),
        &market,
        price::total_of(Coin::<TestCurrencyA>::new(1)).is(Coin::<DEN0>::new(1)),
    )
    .unwrap();
    feed_price(
        deps.as_mut(),
        &market,
        price::total_of(Coin::<TestCurrencyC>::new(1)).is(Coin::<TestCurrencyD>::new(3)),
    )
    .unwrap();

    feed_price(
        deps.as_mut(),
        &market,
        price::total_of(Coin::<TestCurrencyC>::new(1)).is(Coin::<DENX>::new(3)),
    )
    .unwrap();

    feed_price(
        deps.as_mut(),
        &market,
        price::total_of(Coin::<TestCurrencyA>::new(1)).is(Coin::<TestCurrencyB>::new(1)),
    )
    .unwrap();

    feed_price(
        deps.as_mut(),
        &market,
        price::total_of(Coin::<TestCurrencyC>::new(1)).is(Coin::<TestCurrencyD>::new(3)),
    )
    .unwrap();
    feed_price(
        deps.as_mut(),
        &market,
        price::total_of(Coin::<TestCurrencyB>::new(1)).is(Coin::<TestCurrencyC>::new(2)),
    )
    .unwrap();

    feed_price(
        deps.as_mut(),
        &market,
        price::total_of(Coin::<TestCurrencyC>::new(1)).is(Coin::<TestCurrencyB>::new(3)),
    )
    .unwrap();

    feed_price(
        deps.as_mut(),
        &market,
        price::total_of(Coin::<DENZ>::new(1)).is(Coin::<DENX>::new(3)),
    )
    .unwrap();

    feed_price(
        deps.as_mut(),
        &market,
        price::total_of(Coin::<TestCurrencyD>::new(1)).is(Coin::<TestCurrencyA>::new(3)),
    )
    .unwrap();

    feed_price(
        deps.as_mut(),
        &market,
        price::total_of(Coin::<DENC>::new(1)).is(Coin::<TestCurrencyD>::new(3)),
    )
    .unwrap();

    // valid search denom pair
    let query = Parameters::new(MINUTE, 1, mock_env().block.time);
    let price_resp = market
        .price(
            &deps.storage,
            query,
            TestCurrencyA::SYMBOL.to_string(),
            TestCurrencyD::SYMBOL.to_string(),
        )
        .unwrap();
    let expected = price::total_of(Coin::<TestCurrencyA>::new(1)).is(Coin::<TestCurrencyD>::new(6));
    let expected_dto = PriceDTO::try_from(expected).unwrap();

    assert_eq!(expected_dto, price_resp);

    // first and second part of denom pair are the same
    let query = Parameters::new(MINUTE, 1, mock_env().block.time);
    let price_resp = market
        .price(
            &deps.storage,
            query,
            TestCurrencyA::SYMBOL.to_string(),
            TestCurrencyA::SYMBOL.to_string(),
        )
        .unwrap_err();
    assert_eq!(price_resp, PriceFeedsError::InvalidPrice());

    // second part of denome pair doesn't exists in the storage
    let query = Parameters::new(MINUTE, 1, mock_env().block.time);
    assert_eq!(
        market
            .price(
                &deps.storage,
                query,
                TestCurrencyA::SYMBOL.to_string(),
                DENX::SYMBOL.to_string()
            )
            .unwrap_err(),
        PriceFeedsError::UnknownCurrency {}
    );

    // first part of denome pair doesn't exists in the storage
    let query = Parameters::new(MINUTE, 1, mock_env().block.time);
    assert_eq!(
        market
            .price(
                &deps.storage,
                query,
                DENX::SYMBOL.to_string(),
                TestCurrencyA::SYMBOL.to_string()
            )
            .unwrap_err(),
        PriceFeedsError::NoPrice {}
    );
}

fn feed_price<C1, C2>(
    deps: DepsMut,
    market: &PriceFeeds,
    price: Price<C1, C2>,
) -> Result<(), PriceFeedsError>
where
    C1: Currency,
    C2: Currency,
{
    let f_address = deps.api.addr_validate("address1").unwrap();

    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap();
    let ts = Timestamp::from_seconds(now.as_secs());

    let price: PriceDTO = PriceDTO::try_from(price).unwrap();

    market.feed(deps.storage, ts, &f_address, vec![price], MINUTE)?;
    Ok(())
}
