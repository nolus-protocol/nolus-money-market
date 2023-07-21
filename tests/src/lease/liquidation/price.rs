use ::lease::api::ExecuteMsg;
use currency::Currency;
use finance::percent::Percent;
use sdk::{cosmwasm_std::Addr, cw_multi_test::AppResponse};

use crate::{
    common::{self, leaser::Instantiator as LeaserInstantiator, ADMIN, USER},
    lease,
};

use super::{LeaseCoin, LeaseCurrency, LpnCoin, PaymentCurrency, DOWNPAYMENT};

fn liquidation_warning(base: LeaseCoin, quote: LpnCoin, liability: Percent, level: &str) {
    let mut test_case = lease::create_test_case::<PaymentCurrency>();
    let downpayment = lease::create_payment_coin(DOWNPAYMENT);
    let lease_address = lease::open_lease(&mut test_case, downpayment, None);

    common::oracle::feed_price(&mut test_case, Addr::unchecked(ADMIN), base, quote);

    let response: AppResponse = test_case
        .app
        .execute(
            test_case.address_book.oracle().clone(),
            lease_address,
            &ExecuteMsg::PriceAlarm(),
            // &cwcoin::<LeaseCurrency, _>(10000),
            &[],
        )
        .unwrap()
        .unwrap_response();

    let event = response
        .events
        .iter()
        .find(|event| event.ty == "wasm-ls-liquidation-warning")
        .expect("No liquidation warning emitted!");

    let attribute = event
        .attributes
        .iter()
        .find(|attribute| attribute.key == "customer")
        .expect("Customer attribute not present!");

    assert_eq!(attribute.value, USER);

    let attribute = event
        .attributes
        .iter()
        .find(|attribute| attribute.key == "ltv")
        .expect("LTV attribute not present!");

    assert_eq!(attribute.value, liability.units().to_string());

    let attribute = event
        .attributes
        .iter()
        .find(|attribute| attribute.key == "level")
        .expect("Level attribute not present!");

    assert_eq!(attribute.value, level);

    let attribute = event
        .attributes
        .iter()
        .find(|attribute| attribute.key == "lease-asset")
        .expect("Lease Asset attribute not present!");

    assert_eq!(&attribute.value, LeaseCurrency::TICKER);
}

#[test]
#[should_panic = "No liquidation warning emitted!"]
fn liquidation_warning_price_0() {
    liquidation_warning(
        2085713.into(),
        1857159.into(),
        LeaserInstantiator::liability().max(), //not used
        "N/A",
    );
}

#[test]
fn liquidation_warning_price_1() {
    liquidation_warning(
        // ref: 2085713
        2085713.into(),
        // ref: 1857159
        1827159.into(),
        LeaserInstantiator::liability().first_liq_warn(),
        "1",
    );
}

#[test]
fn liquidation_warning_price_2() {
    liquidation_warning(
        // ref: 2085713
        2085713.into(),
        // ref: 1857159
        1757159.into(),
        LeaserInstantiator::liability().second_liq_warn(),
        "2",
    );
}

#[test]
fn liquidation_warning_price_3() {
    liquidation_warning(
        // ref: 2085713
        2085713.into(),
        // ref: 1857159
        1707159.into(),
        LeaserInstantiator::liability().third_liq_warn(),
        "3",
    );
}
