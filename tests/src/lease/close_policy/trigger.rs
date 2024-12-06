use ::lease::api::position::ChangeCmd;
use ::swap::testing::SwapRequest;
use currencies::PaymentGroup;
use finance::{
    coin::{Amount, Coin},
    percent::Percent,
};
use sdk::cosmwasm_std::Addr;

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
    trigger_close(test_case, lease, 999999, 1000000);
}

#[test]
fn trigger_sl() {
    let mut test_case = lease::create_test_case::<PaymentCurrency>();

    let sl = LeaserInstantiator::INITIAL_LTV + Percent::from_permille(1);
    let lease = open_lease(&mut test_case, None, Some(sl));

    // LeaseC/LpnC = 1.01
    trigger_close(test_case, lease, 101, 100);
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

fn trigger_close(mut test_case: LeaseTestCase, lease: Addr, base: Amount, quote: Amount) {
    let mut response = lease::deliver_new_price(
        &mut test_case,
        lease,
        Coin::<LeaseCurrency>::from(base),
        Coin::<LpnCurrency>::from(quote),
    )
    .ignore_response();

    let requests: Vec<SwapRequest<PaymentGroup, PaymentGroup>> = swap::expect_swap(
        &mut response,
        TestCase::DEX_CONNECTION_ID,
        TestCase::LEASE_ICA_ID,
    );
    assert_eq!(1, requests.len());
}
