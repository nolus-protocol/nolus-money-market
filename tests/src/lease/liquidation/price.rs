use currencies::PaymentGroup;
use currency::CurrencyDef as _;
use finance::{
    coin::Amount,
    percent::{Percent100, permilles::Permilles},
    zero::Zero as _,
};
use lease::api::query::StateResponse;
use platform::coin_legacy;
use sdk::{
    cosmwasm_std::{Addr, Event},
    cw_multi_test::AppResponse,
    testing,
};
use swap::testing::SwapRequest;

use crate::{
    common::{
        self, CwCoin, USER, ibc, lease as common_lease,
        leaser::{self, Instantiator as LeaserInstantiator},
        test_case::{TestCase, response::ResponseWithInterChainMsgs},
    },
    lease as lease_mod,
};

use super::super::{DOWNPAYMENT, LeaseCoin, LeaseCurrency, LpnCoin, LpnCurrency, PaymentCurrency};

#[test]
#[should_panic = "No liquidation warning emitted!"]
fn liquidation_warning_price_0() {
    liquidation_warning(
        2085713,
        1857159,
        LeaserInstantiator::liability().max(), //not used
        "N/A",
    );
}

#[test]
fn liquidation_warning_price_1() {
    liquidation_warning(
        // ref: 2085713
        2085713,
        // ref: 1857159
        1827159,
        LeaserInstantiator::FIRST_LIQ_WARN,
        "1",
    );
}

#[test]
fn liquidation_warning_price_2() {
    liquidation_warning(
        // ref: 2085713
        2085713,
        // ref: 1857159
        1757159,
        LeaserInstantiator::SECOND_LIQ_WARN,
        "2",
    );
}

#[test]
fn liquidation_warning_price_3() {
    liquidation_warning(
        // ref: 2085713
        2085713,
        // ref: 1857159
        1707159,
        LeaserInstantiator::THIRD_LIQ_WARN,
        "3",
    );
}

#[test]
fn full_liquidation() {
    let mut test_case = lease_mod::create_test_case::<PaymentCurrency>();

    let lease_addr: Addr = lease_mod::open_lease(&mut test_case, DOWNPAYMENT, None);

    let reserve: Addr = test_case.address_book.reserve().clone();

    let ica_addr: Addr = TestCase::ica_addr(&lease_addr, TestCase::LEASE_ICA_ID);

    let lease_amount: Amount = 2857142857142;
    let borrowed_amount: Amount = 1857142857142;
    let liq_outcome = borrowed_amount - 11123; // to trigger an interaction with Reserve
    test_case.send_funds_from_admin(
        reserve.clone(),
        &[common::cwcoin_from_amount::<LpnCurrency>(
            borrowed_amount - liq_outcome,
        )],
    );

    // the base is chosen to be close to the asset amount to trigger a full liquidation
    let response = lease_mod::deliver_new_price(
        &mut test_case,
        common::coin(lease_amount - 2),
        common::coin(borrowed_amount),
    );

    let requests: Vec<SwapRequest<PaymentGroup, PaymentGroup>> = common::swap::expect_swap(
        response,
        TestCase::DEX_CONNECTION_ID,
        TestCase::LEASE_ICA_ID,
        |_| {},
    );

    let mut response: ResponseWithInterChainMsgs<'_, ()> = common::swap::do_swap(
        &mut test_case.app,
        lease_addr.clone(),
        ica_addr.clone(),
        requests.into_iter(),
        |amount, _, _| {
            assert_eq!(amount, lease_amount);

            liq_outcome
        },
    )
    .ignore_response();

    let transfer_amount: CwCoin = ibc::expect_remote_transfer(
        &mut response,
        TestCase::DEX_CONNECTION_ID,
        TestCase::LEASE_ICA_ID,
    );

    () = response.unwrap_response();

    assert_eq!(
        transfer_amount,
        coin_legacy::to_cosmwasm_on_dex(LpnCoin::new(liq_outcome))
    );

    let response: AppResponse = ibc::do_transfer(
        &mut test_case.app,
        ica_addr,
        lease_addr.clone(),
        true,
        &transfer_amount,
    )
    .unwrap_response();

    response.assert_event(
        &Event::new("wasm-ls-liquidation")
            .add_attribute("payment-amount", borrowed_amount.to_string())
            .add_attribute("loan-close", "true"),
    );
    assert!(
        platform::bank::balance::<LpnCurrency>(&reserve, test_case.app.query())
            .unwrap()
            .is_zero()
    );

    common_lease::assert_lease_balance_eq(
        &test_case.app,
        &lease_addr,
        common::cwcoin(LeaseCoin::ZERO),
    );

    let state = lease_mod::state_query(&test_case, lease_addr);
    assert!(
        matches!(state, StateResponse::Liquidated()),
        "should have been in Liquidated state"
    );
    leaser::assert_no_leases(
        &test_case.app,
        test_case.address_book.leaser().clone(),
        testing::user(USER),
    )
}

fn liquidation_warning(base: Amount, quote: Amount, liability: Percent100, level: &str) {
    let mut test_case = lease_mod::create_test_case::<PaymentCurrency>();
    let _lease = lease_mod::open_lease(&mut test_case, DOWNPAYMENT, None);

    let response: AppResponse = lease_mod::deliver_new_price(
        &mut test_case,
        common::coin::<LeaseCurrency>(base),
        common::coin::<LpnCurrency>(quote),
    )
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

    assert_eq!(attribute.value, testing::user(USER).to_string());

    let attribute = event
        .attributes
        .iter()
        .find(|attribute| attribute.key == "ltv")
        .expect("LTV attribute not present!");

    assert_eq!(attribute.value, Permilles::from(liability).to_string());

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

    assert_eq!(&attribute.value, LeaseCurrency::ticker());
}
