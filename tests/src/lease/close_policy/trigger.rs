use ::lease::api::position::ChangeCmd;
use finance::{coin::Amount, percent::Percent100};
use remote_lease::swap::SwapParams;
use sdk::{
    cosmwasm_std::{Addr, Event},
    cw_multi_test::AppResponse,
};

use crate::{
    common::{
        self,
        remote_lease_controller_stub::{self as stub, ResponseMode, op_tag},
        swap,
    },
    lease::{self, DOWNPAYMENT, LeaseCurrency, LeaserInstantiator, LpnCurrency, PaymentCurrency},
};

use super::LeaseTestCase;

#[test]
fn trigger_tp() {
    let mut test_case = lease::create_test_case::<PaymentCurrency>();

    let tp = LeaserInstantiator::INITIAL_LTV;
    let lease = open_lease(&mut test_case, Some(tp), None);

    // LeaseC/LpnC = 0.999999
    trigger_close(test_case, lease, 999999, 1000000, "take-profit-ltv", tp);
}

#[test]
fn trigger_sl() {
    let mut test_case = lease::create_test_case::<PaymentCurrency>();

    let sl = LeaserInstantiator::INITIAL_LTV + Percent100::from_permille(1);
    let lease = open_lease(&mut test_case, None, Some(sl));

    // LeaseC/LpnC = 1.01
    trigger_close(test_case, lease, 101, 100, "stop-loss-ltv", sl);
}

fn open_lease(
    test_case: &mut LeaseTestCase,
    tp: Option<Percent100>,
    sl: Option<Percent100>,
) -> Addr {
    // LeaseC/LpnC = 1
    let lease = lease::open_lease(test_case, DOWNPAYMENT, None);

    super::change_ok(
        test_case,
        lease.clone(),
        tp.map(ChangeCmd::Set),
        sl.map(ChangeCmd::Set),
    );
    lease
}

fn trigger_close(
    mut test_case: LeaseTestCase,
    lease: Addr,
    base: Amount,
    quote: Amount,
    exp_strategy_key: &str,
    exp_ltv: Percent100,
) {
    let controller = test_case.address_book.remote_lease_controller().clone();
    // Hold the auto-close swap pending so this test observes only the trigger:
    // exactly one swap request and the auto-close event.
    stub::set_response_mode(
        &mut test_case.app,
        &controller,
        op_tag::SWAP,
        ResponseMode::Delayed,
    );

    // Count only the swap the auto-close emits, not the earlier opening swap.
    let swaps_before = swap::count(&test_case.app, &controller);
    let response = lease::deliver_new_price(
        &mut test_case,
        common::coin::<LeaseCurrency>(base),
        common::coin::<LpnCurrency>(quote),
    );

    // The auto-close emits a single sell-asset swap (held pending), so the
    // price-delivery response carries only the auto-close event.
    let app_response = response.unwrap_response();
    assert_events(&app_response, &lease, exp_strategy_key, exp_ltv);

    let captured = swap::captured(&test_case.app, &controller);
    assert!(
        matches!(captured, SwapParams::One { .. }),
        "auto-close must emit exactly one single-coin swap, got {captured:?}",
    );
    assert_eq!(
        swaps_before + 1,
        swap::count(&test_case.app, &controller),
        "the auto-close must emit exactly one swap request",
    );
}

fn assert_events(resp: &AppResponse, lease: &Addr, exp_strategy_key: &str, exp_ltv: Percent100) {
    platform::tests::assert_event(
        &resp.events,
        &Event::new("wasm-ls-auto-close-position")
            .add_attribute("to", lease)
            .add_attribute(exp_strategy_key, exp_ltv.display_primitive()),
    );
}
