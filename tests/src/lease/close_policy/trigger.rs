use ::lease::api::position::ChangeCmd;
use currencies::PaymentGroup;
use finance::{coin::Amount, percent::Percent100};
use sdk::{
    cosmwasm_std::{Addr, Event},
    cw_multi_test::AppResponse,
};

use crate::{
    common::{self, test_case::response::RemoteChain as _},
    lease::{self, DOWNPAYMENT, LeaseCurrency, LeaserInstantiator, LpnCurrency, PaymentCurrency},
};

use super::LeaseTestCase;

#[test]
fn trigger_tp() {
    let mut test_case = lease::create_test_case::<PaymentCurrency>();

    let tp = LeaserInstantiator::INITIAL_LTV;
    let lease = open_lease(&mut test_case, Some(tp), None);

    // LeaseC/LpnC = 1.18 drives the ~76.5% at-open LTV below the 65% TP
    trigger_close(test_case, lease, 100, 118, "take-profit-ltv", tp);
}

#[test]
fn trigger_sl() {
    let mut test_case = lease::create_test_case::<PaymentCurrency>();

    // the literal-floor opening puts the at-open LTV at ~76.5%; a stop
    // loss below that would trigger at set time
    let sl = Percent100::from_percent(78);
    let lease = open_lease(&mut test_case, None, Some(sl));

    // LeaseC/LpnC = 100/103 drives the LTV to ~78.8%, past the stop loss
    trigger_close(test_case, lease, 103, 100, "stop-loss-ltv", sl);
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
    // The auto-close swap now rides the controller, so the price alarm emits
    // no ICA `SwapExactIn`; `unwrap_response` would panic on a non-empty ICA
    // queue. The auto-close event is part of the same response.
    let mut response = lease::deliver_new_price(
        &mut test_case,
        common::coin::<LeaseCurrency>(base),
        common::coin::<LpnCurrency>(quote),
    );
    response.expect_empty();
    let app_response = response.unwrap_response();
    assert_events(&app_response, &lease, exp_strategy_key, exp_ltv);

    // the auto-close added one swap (selling the position asset for LPN) on
    // top of the two opening swaps already recorded for the lease
    let swaps = common::remote_lease_controller_stub::recorded_swaps(
        &test_case.app,
        test_case.address_book.remote_lease_controller(),
        &lease,
    );
    assert_eq!(3, swaps.len());
    assert_eq!(
        currency::dto::<LpnCurrency, PaymentGroup>(),
        swaps.last().expect("the close swap").min_out().currency()
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
