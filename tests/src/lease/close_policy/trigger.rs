use ::lease::api::position::ChangeCmd;
use ::swap::testing::SwapRequest;
use currencies::PaymentGroup;
use finance::{
    coin::Amount,
    percent::{Percent100, permilles::Permilles},
};
use sdk::{
    cosmwasm_std::{Addr, Event},
    cw_multi_test::AppResponse,
};

use crate::{
    common::{self, swap},
    lease::{
        self, DOWNPAYMENT, LeaseCurrency, LeaserInstantiator, LpnCurrency, PaymentCurrency,
        TestCase,
    },
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
    let response = lease::deliver_new_price(
        &mut test_case,
        common::coin::<LeaseCurrency>(base),
        common::coin::<LpnCurrency>(quote),
    );

    let requests: Vec<SwapRequest<PaymentGroup, PaymentGroup>> = swap::expect_swap(
        response,
        TestCase::DEX_CONNECTION_ID,
        TestCase::LEASE_ICA_ID,
        |app_response| {
            assert_events(app_response, &lease, exp_strategy_key, exp_ltv);
        },
    );
    assert_eq!(1, requests.len());
}

fn assert_events(resp: &AppResponse, lease: &Addr, exp_strategy_key: &str, exp_ltv: Percent100) {
    platform::tests::assert_event(
        &resp.events,
        &Event::new("wasm-ls-auto-close-position")
            .add_attribute("to", lease)
            .add_attribute(exp_strategy_key, Permilles::from(exp_ltv).to_string()),
    );
}
