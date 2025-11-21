use std::slice;

use currencies::{
    Lpn,
    testing::{LeaseC1, LeaseC2, LeaseC3, LeaseC7, PaymentC1},
};
use currency::CurrencyDef;
use finance::{
    coin::{Amount, Coin},
    percent::Percent100,
    price::{self, Price},
};
use sdk::{
    cosmwasm_std::{Addr, coin},
    cw_multi_test::ContractWrapper,
    testing,
};

use crate::common::{
    self, ADDON_OPTIMAL_INTEREST_RATE, BASE_INTEREST_RATE, USER, UTILIZATION_OPTIMAL,
    leaser as leaser_mod,
    lpp::{self as lpp_mod},
    oracle as oracle_mod,
    protocols::Registry,
    test_case::{TestCase, builder::BlankBuilder as TestCaseBuilder},
};

type TheCurrency = Lpn;

#[test]
fn test_quote() {
    type Lpn = TheCurrency;
    type Downpayment = Lpn;
    type LeaseCurrency = LeaseC7;

    let mut test_case = TestCaseBuilder::<Lpn>::new()
        .init_lpp(
            None,
            BASE_INTEREST_RATE,
            UTILIZATION_OPTIMAL,
            ADDON_OPTIMAL_INTEREST_RATE,
            TestCase::DEFAULT_LPP_MIN_UTILIZATION,
        )
        .init_time_alarms()
        .init_protocols_registry(Registry::NoProtocol)
        .init_oracle(None)
        .init_treasury()
        .init_profit(24)
        .init_reserve()
        .init_leaser()
        .into_generic();

    test_case.send_funds_from_admin(
        testing::user(USER),
        &[common::cwcoin_from_amount::<Lpn>(500)],
    );

    let price_lease_lpn: Price<LeaseCurrency, Lpn> =
        price::total_of(common::coin(2)).is(common::coin(1));
    let feeder = setup_feeder(&mut test_case);
    oracle_mod::feed_price(
        &mut test_case,
        feeder,
        Coin::<LeaseCurrency>::new(2),
        Coin::<Lpn>::new(1),
    );

    let leaser = test_case.address_book.leaser().clone();
    let downpayment = Coin::new(100);
    let borrow = Coin::<Lpn>::new(185);
    let resp = leaser_mod::query_quote::<Downpayment, LeaseCurrency>(
        &test_case.app,
        leaser,
        downpayment,
        None,
    );

    assert_eq!(resp.borrow.try_into(), Ok(borrow));
    assert_eq!(
        resp.total.try_into(),
        Ok(price::total(downpayment + borrow, price_lease_lpn.inv()).unwrap())
    );

    /*   TODO: test with different time periods and amounts in LPP
     */

    assert_eq!(resp.annual_interest_rate, Percent100::from_permille(72),);

    assert_eq!(
        resp.annual_interest_rate_margin,
        Percent100::from_permille(30),
    );

    let leaser = test_case.address_book.leaser().clone();
    let resp = leaser_mod::query_quote::<Downpayment, LeaseCurrency>(
        &test_case.app,
        leaser,
        Coin::new(15),
        None,
    );

    assert_eq!(resp.borrow.try_into(), Ok(Coin::<Lpn>::new(27)));
    assert_eq!(
        resp.total.try_into(),
        Ok(Coin::<LeaseCurrency>::new(15 * 2 + 27 * 2))
    );
}

fn common_quote_with_conversion(
    downpayment: Coin<PaymentC1>,
    borrow_after_mul2: Coin<TheCurrency>,
) {
    type Lpn = TheCurrency;
    type LeaseCurrency = LeaseC2;

    const LPNS: Amount = 5_000_000_000_000;
    const OSMOS: Amount = 5_000_000_000_000;
    const CROS: Amount = 5_000_000_000_000;

    const USER_ATOMS: Amount = 5_000_000_000;

    let lpp_reserve = vec![
        common::cwcoin_from_amount::<Lpn>(LPNS),
        common::cwcoin_from_amount::<LeaseC3>(OSMOS),
        common::cwcoin_from_amount::<LeaseCurrency>(CROS),
    ];

    let user_reserve = common::cwcoin_from_amount::<LeaseC1>(USER_ATOMS);

    let user_addr = testing::user(USER);

    let mut test_case = TestCaseBuilder::<Lpn>::with_reserve(&{
        let mut reserve = vec![common::cwcoin_from_amount::<Lpn>(1_000_000_000)];

        reserve.extend_from_slice(&lpp_reserve);

        reserve.extend_from_slice(slice::from_ref(&user_reserve));

        reserve
    })
    .init_lpp_with_funds(
        None,
        &[
            coin(LPNS, Lpn::bank()),
            coin(OSMOS, LeaseC3::bank()),
            coin(CROS, LeaseCurrency::bank()),
        ],
        BASE_INTEREST_RATE,
        UTILIZATION_OPTIMAL,
        ADDON_OPTIMAL_INTEREST_RATE,
        TestCase::DEFAULT_LPP_MIN_UTILIZATION,
    )
    .init_time_alarms()
    .init_protocols_registry(Registry::NoProtocol)
    .init_oracle(None)
    .init_treasury()
    .init_profit(24)
    .init_reserve()
    .init_leaser()
    .into_generic();

    test_case.send_funds_from_admin(user_addr, &[user_reserve]);

    let feeder_addr = testing::user("feeder1");

    oracle_mod::add_feeder(&mut test_case, feeder_addr.clone());

    let dpn_lpn_base = Coin::<PaymentC1>::new(1);
    let dpn_lpn_quote = Coin::<Lpn>::new(2);
    let dpn_lpn_price = price::total_of(dpn_lpn_base).is(dpn_lpn_quote);

    let lpn_asset_base = Coin::<Lpn>::new(1);
    let lpn_asset_quote = Coin::<LeaseCurrency>::new(2);
    let lpn_asset_price = price::total_of(lpn_asset_base).is(lpn_asset_quote);

    oracle_mod::feed_price(
        &mut test_case,
        feeder_addr.clone(),
        dpn_lpn_base,
        dpn_lpn_quote,
    );
    oracle_mod::feed_price(&mut test_case, feeder_addr, lpn_asset_quote, lpn_asset_base);

    let resp = leaser_mod::query_quote::<_, LeaseCurrency>(
        &test_case.app,
        test_case.address_book.leaser().clone(),
        downpayment,
        None,
    );

    assert_eq!(
        resp.borrow.try_into(),
        Ok(borrow_after_mul2),
        "Borrow amount is different!"
    );
    assert_eq!(
        resp.total.try_into(),
        Ok(price::total(
            price::total(downpayment, dpn_lpn_price).unwrap() + borrow_after_mul2,
            lpn_asset_price
        )
        .unwrap()),
        "Total amount is different!"
    );
}

#[test]
fn test_quote_with_conversion_100() {
    common_quote_with_conversion(Coin::new(100), Coin::new(371));
}

#[test]
fn test_quote_with_conversion_200() {
    common_quote_with_conversion(Coin::new(200), Coin::new(742));
}

#[test]
fn test_quote_with_conversion_5000() {
    common_quote_with_conversion(Coin::new(5000), Coin::new(18571));
}

#[test]
fn test_quote_fixed_rate() {
    type Lpn = TheCurrency;
    type Downpayment = Lpn;
    type LeaseCurrency = LeaseC2;

    let mut test_case = TestCaseBuilder::<Lpn>::new()
        .init_lpp(
            Some(
                ContractWrapper::new(
                    lpp::contract::execute,
                    lpp::contract::instantiate,
                    lpp_mod::mock_quote_query,
                )
                .with_sudo(lpp::contract::sudo),
            ),
            BASE_INTEREST_RATE,
            UTILIZATION_OPTIMAL,
            ADDON_OPTIMAL_INTEREST_RATE,
            TestCase::DEFAULT_LPP_MIN_UTILIZATION,
        )
        .init_time_alarms()
        .init_protocols_registry(Registry::NoProtocol)
        .init_oracle(None)
        .init_treasury()
        .init_profit(24)
        .init_reserve()
        .init_leaser()
        .into_generic();

    let feeder = setup_feeder(&mut test_case);
    oracle_mod::feed_price(
        &mut test_case,
        feeder,
        Coin::<LeaseCurrency>::new(3),
        Coin::<Lpn>::new(1),
    );
    let resp = leaser_mod::query_quote::<Downpayment, LeaseCurrency>(
        &test_case.app,
        test_case.address_book.leaser().clone(),
        Coin::<Downpayment>::new(100),
        None,
    );

    assert_eq!(resp.borrow.try_into(), Ok(Coin::<Lpn>::new(185)));
    assert_eq!(
        resp.total.try_into(),
        Ok(Coin::<LeaseCurrency>::new(100 * 3 + 185 * 3))
    );

    /*   TODO: test with different time periods and amounts in LPP
        103% =
        100% lpp annual_interest_rate (when calling the test version of get_annual_interest_rate() in lpp_querier.rs)
        +
        3% margin_interest_rate of the leaser
    */

    assert_eq!(resp.annual_interest_rate, Percent100::HUNDRED);

    assert_eq!(
        resp.annual_interest_rate_margin,
        Percent100::from_percent(3)
    );
}

fn setup_feeder<ProtocolsRegistry, Treasury, Profit, Reserve, Leaser, Lpp, TimeAlarms>(
    test_case: &mut TestCase<
        ProtocolsRegistry,
        Treasury,
        Profit,
        Reserve,
        Leaser,
        Lpp,
        Addr,
        TimeAlarms,
    >,
) -> Addr {
    let feeder = testing::user("feeder_main");
    oracle_mod::add_feeder(test_case, feeder.clone());
    feeder
}
