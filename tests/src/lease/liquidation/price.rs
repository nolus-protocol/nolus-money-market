use currencies::PaymentGroup;
use currency::CurrencyDef as _;
use finance::{
    coin::{Amount, CoinDTO},
    percent::Percent100,
    zero::Zero as _,
};
use lease::api::query::StateResponse;
use sdk::{
    cosmwasm_std::{Addr, Event},
    cw_multi_test::AppResponse,
    testing,
};

use crate::{
    common::{
        self, USER, lease as common_lease,
        leaser::{self, Instantiator as LeaserInstantiator},
        remote_lease_controller_stub::SwapFill,
        swap,
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

    let lease_amount: Amount = 2857142857142;
    let borrowed_amount: Amount = 1857142857142;
    let liq_outcome = borrowed_amount - 11123; // to trigger an interaction with Reserve
    test_case.send_funds_from_admin(
        reserve.clone(),
        &[common::cwcoin_from_amount::<LpnCurrency>(
            borrowed_amount - liq_outcome,
        )],
    );

    let controller = test_case.address_book.remote_lease_controller().clone();
    // The bad-fill outcome is smaller than the sold collateral, forcing the
    // reserve to cover the shortfall — a fixed absolute swap output.
    swap::set_fill(
        &mut test_case.app,
        &controller,
        SwapFill::Fixed(liq_outcome),
    );

    // the base is chosen to be close to the asset amount to trigger a full liquidation
    let response = lease_mod::deliver_new_price(
        &mut test_case,
        common::coin(lease_amount - 2),
        common::coin(borrowed_amount),
    );

    // The liquidation sell-asset swap and the proceeds transfer-out fired inline
    // on the price delivery; the lease (local-output) is now in TransferInFinish,
    // polling its bank balance for the returning LPN proceeds.
    let _ = response.unwrap_response();

    // Fidelity: the swap input is the full liquidated collateral.
    let captured = swap::captured(&test_case.app, &controller);
    assert_eq!(
        Into::<CoinDTO<PaymentGroup>>::into(common::coin::<LeaseCurrency>(lease_amount)),
        swap::token_in(&captured),
    );

    // Fidelity: the emitted transfer-out returns exactly the LPN proceeds.
    assert_eq!(
        Into::<CoinDTO<PaymentGroup>>::into(LpnCoin::new(liq_outcome)),
        swap::captured_transfer_out(&test_case.app, &controller),
    );

    let time_alarms = test_case.address_book.time_alarms().clone();
    let response: AppResponse = swap::deliver_transfer_in(
        &mut test_case.app,
        time_alarms,
        lease_addr.clone(),
        &common::cwcoin(LpnCoin::new(liq_outcome)),
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

    assert_eq!(attribute.value, liability.display_primitive());

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
