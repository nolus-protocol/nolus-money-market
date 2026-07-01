//! Funding E2E for the remote-lease opening (nolus-protocol/nolus-money-market#647).
//!
//! The opening funds the lease by ICS-20-transferring the downpayment and the
//! principal to the lease's Solana-side `LeaseAuthority` over the paired
//! transfer channel — there is no Interchain Account on the opening path. Each
//! coin rides its own packet, one in flight at a time; the last
//! acknowledgment is the arrival gate that releases the opening swaps.
//!
//! Drivers:
//!
//! - `funding_addresses_the_lease_authority` — both funding transfers are
//!   addressed to the `LeaseAuthority` the `OpenLease` ack returned (the same
//!   value the `Funding` query reports), and the two acks drive the opening to
//!   `Opened`.
//! - `partial_arrival_holds_the_gate` — with the downpayment acked but the
//!   principal still in flight, the opening stays in the funding leg; only the
//!   last ack releases the gate.
//! - `funding_timeout_re_emits_one_coin` — a transfer timeout re-emits the
//!   single in-flight coin verbatim (no double-send of an already-scheduled
//!   coin), then the opening still completes.
//! - `funding_heal_re_emits_the_in_flight_coin` — a permissionless `Heal`
//!   re-emits the in-flight coin, and the opening completes.
//! - `pre_funding_failure_moves_no_funds` — an `OpenLease` error refunds the
//!   downpayment and emits no funding transfer at all (the refund path is
//!   reachable only before any coin is sent).
//! - `opening_emits_no_interchain_account` — the opening emits a funding
//!   transfer over the paired channel and opens no interchain account.

use crate::common::testing;
use currency::CurrencyDef;
use finance::coin::Coin;
use lease::api::{
    ExecuteMsg,
    query::{StateResponse, opening::OngoingTrx as OpeningOngoingTrx},
};
use platform::coin_legacy;
use remote_lease::callback::RemoteErrorMessage;
use sdk::cosmwasm_std::{Addr, Coin as CwCoin, Event};

use crate::common::{
    self, USER,
    remote_lease_controller_stub::{self as stub, ResponseMode, op_tag},
    test_case::{TestCase, response::RemoteChain as _},
};

use super::{DOWNPAYMENT, LeaseTestCase, LpnCurrency, PaymentCoin, PaymentCurrency};

const OPENING_SWAP_EVENT: &str = "wasm-ls-open-swap";
const OPEN_FAILED_EVENT: &str = "wasm-ls-remote-lease-open-failed";

#[test]
fn funding_addresses_the_lease_authority() {
    let mut test_case = super::create_test_case::<PaymentCurrency>();
    let (lease, sender, receiver, downpayment_cw) =
        open_capturing_funding(&mut test_case, DOWNPAYMENT);

    // The downpayment rides the paired transfer channel, sent by the lease to
    // the `LeaseAuthority` (not an ICA host), and matches the query's receiver.
    assert_eq!(sender, lease.as_str());
    assert!(receiver.starts_with("StubPda"), "got receiver {receiver:?}");
    assert_eq!(receiver, funding_receiver(&test_case, &lease));
    assert_eq!(
        downpayment_cw,
        coin_legacy::to_cosmwasm_on_nolus(DOWNPAYMENT)
    );

    let exp_borrow = super::quote_borrow(&test_case, DOWNPAYMENT);
    let ica_addr = TestCase::ica_addr(&lease, TestCase::LEASE_ICA_ID);

    // ack the downpayment; the principal is scheduled next, addressed to the
    // very same `LeaseAuthority`.
    let mut after_downpayment = common::ibc::do_transfer(
        &mut test_case.app,
        lease.clone(),
        ica_addr.clone(),
        false,
        &downpayment_cw,
    );
    let (principal_sender, principal_receiver, principal_cw) =
        after_downpayment.take_ibc_transfer(TestCase::LEASER_IBC_CHANNEL);
    assert_eq!(principal_sender, lease.as_str());
    assert_eq!(principal_receiver, receiver);
    assert_eq!(principal_cw, coin_legacy::to_cosmwasm_on_nolus(exp_borrow));
    () = after_downpayment.ignore_response().unwrap_response();

    // ack the principal; the last ack releases the gate and the opening swaps
    // settle inline through the controller stand-in.
    () = common::ibc::do_transfer(
        &mut test_case.app,
        lease.clone(),
        ica_addr,
        false,
        &principal_cw,
    )
    .ignore_response()
    .unwrap_response();

    assert!(
        matches!(
            super::state_query(&test_case, lease),
            StateResponse::Opened { .. }
        ),
        "the opening must settle once both coins have arrived",
    );
}

#[test]
fn partial_arrival_holds_the_gate() {
    let mut test_case = super::create_test_case::<PaymentCurrency>();
    let lease = super::try_init_lease(&mut test_case, DOWNPAYMENT, None);
    let exp_borrow = super::quote_borrow(&test_case, DOWNPAYMENT);
    let ica_addr = TestCase::ica_addr(&lease, TestCase::LEASE_ICA_ID);

    let downpayment_cw = coin_legacy::to_cosmwasm_on_nolus(DOWNPAYMENT);
    let borrow_cw = coin_legacy::to_cosmwasm_on_nolus(exp_borrow);

    // ack the downpayment only: the principal is now in flight
    let mut after_downpayment = common::ibc::do_transfer(
        &mut test_case.app,
        lease.clone(),
        ica_addr.clone(),
        false,
        &downpayment_cw,
    );
    let (_sender, _receiver, principal) =
        after_downpayment.take_ibc_transfer(TestCase::LEASER_IBC_CHANNEL);
    assert_eq!(principal, borrow_cw);
    () = after_downpayment.ignore_response().unwrap_response();

    // the gate must hold: the opening stays in the funding leg, not the swap leg
    assert!(
        matches!(
            super::state_query(&test_case, lease.clone()),
            StateResponse::Opening {
                in_progress: OpeningOngoingTrx::Funding { .. },
                ..
            }
        ),
        "a partial arrival must not release the opening swaps",
    );

    // the principal's ack releases the gate
    () = common::ibc::do_transfer(
        &mut test_case.app,
        lease.clone(),
        ica_addr,
        false,
        &borrow_cw,
    )
    .ignore_response()
    .unwrap_response();

    assert!(matches!(
        super::state_query(&test_case, lease),
        StateResponse::Opened { .. }
    ));
}

#[test]
fn funding_timeout_re_emits_one_coin() {
    let mut test_case = super::create_test_case::<PaymentCurrency>();
    let lease = super::try_init_lease(&mut test_case, DOWNPAYMENT, None);
    let downpayment_cw = coin_legacy::to_cosmwasm_on_nolus(DOWNPAYMENT);

    // the downpayment is in flight; a transfer timeout must re-emit exactly
    // that one coin - never the whole batch (which would double-send an
    // already-scheduled coin)
    let mut timed_out = common::ibc::timeout_transfer(&mut test_case.app, lease.clone());
    let (_sender, _receiver, re_emitted) =
        timed_out.take_ibc_transfer(TestCase::LEASER_IBC_CHANNEL);
    assert_eq!(
        re_emitted, downpayment_cw,
        "the timeout must re-emit the in-flight downpayment verbatim",
    );
    let timeout_events = timed_out.unwrap_response();
    expect_attribute(
        &timeout_events.events,
        OPENING_SWAP_EVENT,
        "timeout",
        "retry",
    );

    // the opening still in the funding leg, and a normal drive completes it
    let exp_borrow = super::quote_borrow(&test_case, DOWNPAYMENT);
    let ica_addr = TestCase::ica_addr(&lease, TestCase::LEASE_ICA_ID);
    let _response = common::lease::fund_remote_lease::<PaymentCurrency, LpnCurrency>(
        &mut test_case.app,
        lease.clone(),
        ica_addr,
        (DOWNPAYMENT, exp_borrow),
    )
    .unwrap_response();

    assert!(matches!(
        super::state_query(&test_case, lease),
        StateResponse::Opened { .. }
    ));
}

#[test]
fn funding_heal_re_emits_the_in_flight_coin() {
    let mut test_case = super::create_test_case::<PaymentCurrency>();
    let lease = super::try_init_lease(&mut test_case, DOWNPAYMENT, None);
    let downpayment_cw = coin_legacy::to_cosmwasm_on_nolus(DOWNPAYMENT);

    // a permissionless heal re-emits the single in-flight coin verbatim
    let mut healed = test_case
        .app
        .execute(testing::user(USER), lease.clone(), &ExecuteMsg::Heal(), &[])
        .expect("heal must re-emit the in-flight funding coin");
    let (_sender, _receiver, re_emitted) = healed.take_ibc_transfer(TestCase::LEASER_IBC_CHANNEL);
    assert_eq!(re_emitted, downpayment_cw);
    let heal_events = healed.unwrap_response();
    expect_attribute(&heal_events.events, OPENING_SWAP_EVENT, "heal", "re-emit");

    let exp_borrow = super::quote_borrow(&test_case, DOWNPAYMENT);
    let ica_addr = TestCase::ica_addr(&lease, TestCase::LEASE_ICA_ID);
    let _response = common::lease::fund_remote_lease::<PaymentCurrency, LpnCurrency>(
        &mut test_case.app,
        lease.clone(),
        ica_addr,
        (DOWNPAYMENT, exp_borrow),
    )
    .unwrap_response();

    assert!(matches!(
        super::state_query(&test_case, lease),
        StateResponse::Opened { .. }
    ));
}

#[test]
fn pre_funding_failure_moves_no_funds() {
    let mut test_case = super::create_test_case::<PaymentCurrency>();
    let controller = test_case.address_book.remote_lease_controller().clone();
    let reason = RemoteErrorMessage::new("solana side rejected").expect("within length cap");
    stub::set_response_mode(
        &mut test_case.app,
        &controller,
        op_tag::OPEN_LEASE,
        ResponseMode::Err(reason),
    );

    let customer = testing::user(USER);
    let balance_before = balance::<PaymentCurrency>(&test_case, &customer);

    // The `OpenLease` ack fails before any coin is sent: the refund path must
    // run and `unwrap_response` proves no funding transfer was emitted.
    let response = test_case
        .app
        .execute(
            customer.clone(),
            test_case.address_book.leaser().clone(),
            &leaser::msg::ExecuteMsg::OpenLease {
                currency: currency::dto::<super::LeaseCurrency, _>(),
                max_ltd: None,
            },
            &[common::cwcoin(DOWNPAYMENT)],
        )
        .unwrap()
        .unwrap_response();

    assert!(
        response
            .events
            .iter()
            .any(|event| event.ty == OPEN_FAILED_EVENT),
        "a pre-funding failure must emit the open-failed event",
    );
    assert_eq!(
        balance_before,
        balance::<PaymentCurrency>(&test_case, &customer),
        "the downpayment must be refunded in full when funding never starts",
    );
}

#[test]
fn opening_emits_no_interchain_account() {
    let mut test_case = super::create_test_case::<PaymentCurrency>();

    let mut response = test_case
        .app
        .execute(
            testing::user(USER),
            test_case.address_book.leaser().clone(),
            &leaser::msg::ExecuteMsg::OpenLease {
                currency: currency::dto::<super::LeaseCurrency, _>(),
                max_ltd: None,
            },
            &[common::cwcoin(DOWNPAYMENT)],
        )
        .unwrap();

    // The opening's only interchain message is the funding transfer over the
    // paired channel, addressed to the `LeaseAuthority`; `unwrap_response`
    // asserts nothing else was emitted - so no interchain account is ever
    // opened on this path.
    let (_sender, receiver, _token) = response.take_ibc_transfer(TestCase::LEASER_IBC_CHANNEL);
    assert!(receiver.starts_with("StubPda"), "got receiver {receiver:?}");
    () = response.ignore_response().unwrap_response();
}

/// Open a lease through the leaser and capture the downpayment funding transfer
/// the synchronous `OpenLease` ack emits inline. Returns the lease and the
/// transfer's `(sender, receiver, token)`; the lease is left in the funding leg
/// with the downpayment in flight.
fn open_capturing_funding(
    test_case: &mut LeaseTestCase,
    downpayment: PaymentCoin,
) -> (Addr, String, String, CwCoin) {
    let mut response = test_case
        .app
        .execute(
            testing::user(USER),
            test_case.address_book.leaser().clone(),
            &leaser::msg::ExecuteMsg::OpenLease {
                currency: currency::dto::<super::LeaseCurrency, _>(),
                max_ltd: None,
            },
            &[common::cwcoin(downpayment)],
        )
        .unwrap();

    let (sender, receiver, token) = response.take_ibc_transfer(TestCase::LEASER_IBC_CHANNEL);
    () = response.ignore_response().unwrap_response();

    let lease = common::leaser::expect_a_lease(
        &test_case.app,
        test_case.address_book.leaser().clone(),
        testing::user(USER),
    );
    (lease, sender, receiver, token)
}

/// The `LeaseAuthority` the funding leg reports it is sending to.
fn funding_receiver(test_case: &LeaseTestCase, lease: &Addr) -> String {
    match super::state_query(test_case, lease.clone()) {
        StateResponse::Opening {
            in_progress: OpeningOngoingTrx::Funding { receiver },
            ..
        } => receiver,
        other => panic!("expected the funding leg, got {other:?}"),
    }
}

fn balance<C>(test_case: &LeaseTestCase, account: &Addr) -> Coin<C>
where
    C: CurrencyDef,
{
    use platform::bank::{self, BankAccountView};
    bank::account_view(account, test_case.app.query())
        .balance::<C>()
        .expect("balance query must succeed")
}

fn expect_attribute(events: &[Event], event_type: &str, key: &str, value: &str) {
    assert!(
        events.iter().any(|event| {
            event.ty == event_type
                && event
                    .attributes
                    .iter()
                    .any(|attr| attr.key == key && attr.value == value)
        }),
        "expected event `{event_type}` with `{key} = {value}`, got {events:?}",
    );
}
