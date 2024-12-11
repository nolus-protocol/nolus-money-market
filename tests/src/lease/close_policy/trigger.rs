use ::lease::api::position::ChangeCmd;
use ::swap::testing::SwapRequest;
use currencies::PaymentGroup;
use finance::{
    coin::{Amount, Coin},
    percent::Percent,
};
use sdk::{
    cosmwasm_std::{Addr, Event},
    cw_multi_test::AppResponse,
};

use crate::{
    common::swap,
    lease::{
        self, LeaseCurrency, LeaserInstantiator, LpnCurrency, PaymentCurrency, TestCase,
        DOWNPAYMENT,
    },
};

use super::LeaseTestCase;

#[test]
fn trigger_tp() {
    let mut test_case = lease::create_test_case::<PaymentCurrency>();

    let tp = LeaserInstantiator::INITIAL_LTV;
    let lease = open_lease(&mut test_case, Some(tp), None);

    // LeaseC/LpnC = 0.999999
    let resp = trigger_close(test_case, 999999, 1000000);
    assert_events(&resp, &lease, "take-profit-ltv", tp);
}

#[test]
fn trigger_sl() {
    let mut test_case = lease::create_test_case::<PaymentCurrency>();

    let sl = LeaserInstantiator::INITIAL_LTV + Percent::from_permille(1);
    let lease = open_lease(&mut test_case, None, Some(sl));

    // LeaseC/LpnC = 1.01
    let resp = trigger_close(test_case, 101, 100);
    assert_events(&resp, &lease, "stop-loss-ltv", sl);
}

fn open_lease(test_case: &mut LeaseTestCase, tp: Option<Percent>, sl: Option<Percent>) -> Addr {
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

fn trigger_close(mut test_case: LeaseTestCase, base: Amount, quote: Amount) -> AppResponse {
    let mut response = lease::deliver_new_price(
        &mut test_case,
        Coin::<LeaseCurrency>::from(base),
        Coin::<LpnCurrency>::from(quote),
    );

    let requests: Vec<SwapRequest<PaymentGroup, PaymentGroup>> = swap::expect_swap(
        &mut response,
        TestCase::DEX_CONNECTION_ID,
        TestCase::LEASE_ICA_ID,
    );
    assert_eq!(1, requests.len());

    response.unwrap_response()
}

fn assert_events(resp: &AppResponse, lease: &Addr, exp_strategy_key: &str, exp_ltv: Percent) {
    platform::tests::assert_event(
        &resp.events,
        &Event::new("wasm-ls-auto-close-position")
            .add_attribute("to", lease)
            .add_attribute(exp_strategy_key, exp_ltv.units().to_string()),
    );
}
