//! CloseLease (remote-account close) E2E (nolus-protocol/ibc-solray#142,
//! close leg).
//!
//! The customer payout comes FIRST; `CloseLease` — closing the
//! Solana-side lease account — runs afterwards, best-effort. When the
//! drain's funds-arrival poll finds all coins landed, the finish
//! transaction does, in one response: the customer payout bank-send, a
//! `CloseLease` execute to the remote-lease controller as a
//! reply-on-error submessage, and the leaser finalize. The lease then
//! sits in `ClosingRemoteLease` until the close acknowledgment lands:
//!
//! - `OperationOk(CloseLease)` completes to the `Closed` terminal,
//! - `OperationErr` and a synchronous controller failure are absorbed
//!   (payout and finalize stay committed) and remain recoverable via the
//!   permissionless `Heal`, which re-emits the `CloseLease` verbatim,
//! - `OperationTimeout` re-emits the `CloseLease` verbatim,
//! - stale time/price alarms are ignored; a stranger's callback errors,
//! - the state query deliberately reports `Closed()` already — progress
//!   is asserted via events and the stub's close recorder, never the
//!   state query,
//! - once `Closed`, a late `CloseLease` acknowledgment is absorbed by
//!   the terminal's late-ack absorber.

use crate::common::testing;
use access_control::error::Error as AccessError;
use lease::{
    api::{ExecuteMsg, query::StateResponse},
    error::ContractError,
};
use remote_lease::{
    callback::{RemoteErrorMessage, RemoteLeaseCallback, RemoteOperationOutcome},
    response::{CloseLeaseResponse, WireOperationResponse},
};
use sdk::{
    cosmwasm_std::{Addr, Event},
    cw_multi_test::AppResponse,
};

use crate::common::{
    USER, leaser as leaser_mod,
    remote_lease_controller_stub::{self as stub, ResponseMode, op_tag},
};

use super::{LeaseCoin, LeaseTestCase, PaymentCurrency, repay};

const CLOSING_REMOTE_LEASE_EVENT: &str = "wasm-ls-close-remote-lease";
const LATE_ACK_EVENT: &str = "wasm-ls-remote-lease-late-ack";
const PAYOUT_EVENT: &str = "wasm-ls-close";

#[test]
fn close_lifecycle_happy_path() {
    let mut test_case = super::create_test_case::<PaymentCurrency>();
    let controller = test_case.address_book.remote_lease_controller().clone();

    let (lease, finish_response) = open_drain_and_finish(&mut test_case);

    assert_eq!(
        1,
        stub::recorded_closes(&test_case.app, &controller, &lease)
    );
    finish_response.assert_event(&Event::new(PAYOUT_EVENT));
    finish_response.assert_event(&completion_event(&lease));
    assert_eq!(
        StateResponse::Closed(),
        super::state_query(&test_case, lease.clone())
    );
    leaser_mod::assert_no_leases(
        &test_case.app,
        test_case.address_book.leaser().clone(),
        testing::user(USER),
    );
}

#[test]
fn close_error_ack_absorbed_until_heal() {
    let mut test_case = super::create_test_case::<PaymentCurrency>();
    let controller = test_case.address_book.remote_lease_controller().clone();
    let reason =
        RemoteErrorMessage::new("remote account close failure").expect("within length cap");
    stub::set_response_mode(
        &mut test_case.app,
        &controller,
        op_tag::CLOSE_LEASE,
        ResponseMode::Err(reason),
    );

    let (lease, finish_response) = open_drain_and_finish(&mut test_case);

    finish_response.assert_event(
        &Event::new(CLOSING_REMOTE_LEASE_EVENT).add_attribute("absorbed", "remote-error"),
    );
    assert_eq!(
        1,
        stub::recorded_closes(&test_case.app, &controller, &lease)
    );

    stub::set_response_mode(
        &mut test_case.app,
        &controller,
        op_tag::CLOSE_LEASE,
        ResponseMode::Ok,
    );
    let heal_response = super::heal(&mut test_case, lease.clone());
    heal_response
        .assert_event(&Event::new(CLOSING_REMOTE_LEASE_EVENT).add_attribute("heal", "re-emit"));
    heal_response.assert_event(&completion_event(&lease));
    assert_eq!(
        2,
        stub::recorded_closes(&test_case.app, &controller, &lease)
    );

    assert_closed_terminal(&mut test_case, &controller, &lease);
}

#[test]
fn close_sync_failure_does_not_block_payout() {
    let mut test_case = super::create_test_case::<PaymentCurrency>();
    let controller = test_case.address_book.remote_lease_controller().clone();
    stub::set_response_mode(
        &mut test_case.app,
        &controller,
        op_tag::CLOSE_LEASE,
        ResponseMode::FailSync,
    );

    let (lease, finish_response) = open_drain_and_finish(&mut test_case);

    finish_response.assert_event(
        &Event::new(CLOSING_REMOTE_LEASE_EVENT).add_attribute("absorbed", "emission-failed"),
    );
    assert_eq!(
        0,
        stub::recorded_closes(&test_case.app, &controller, &lease)
    );
    leaser_mod::assert_no_leases(
        &test_case.app,
        test_case.address_book.leaser().clone(),
        testing::user(USER),
    );

    stub::set_response_mode(
        &mut test_case.app,
        &controller,
        op_tag::CLOSE_LEASE,
        ResponseMode::Ok,
    );
    let heal_response = super::heal(&mut test_case, lease.clone());
    heal_response.assert_event(&completion_event(&lease));
    assert_eq!(
        1,
        stub::recorded_closes(&test_case.app, &controller, &lease)
    );
}

#[test]
fn close_timeout_reemits() {
    let mut test_case = super::create_test_case::<PaymentCurrency>();
    let controller = test_case.address_book.remote_lease_controller().clone();
    stub::set_response_mode(
        &mut test_case.app,
        &controller,
        op_tag::CLOSE_LEASE,
        ResponseMode::Delayed,
    );

    let (lease, finish_response) = open_drain_and_finish(&mut test_case);

    assert_eq!(
        1,
        stub::recorded_closes(&test_case.app, &controller, &lease)
    );
    assert_no_remote_close_events(&finish_response);

    let timeout_response = stub::inject_callback(
        &mut test_case.app,
        &controller,
        &lease,
        RemoteLeaseCallback {
            nonce: 0,
            outcome: RemoteOperationOutcome::OperationTimeout,
        },
    );
    timeout_response.assert_event(
        &Event::new(CLOSING_REMOTE_LEASE_EVENT)
            .add_attribute("id", lease.clone())
            .add_attribute("timeout", "retry"),
    );
    assert_eq!(
        2,
        stub::recorded_closes(&test_case.app, &controller, &lease)
    );

    // The re-emission overwrote the stub's pending slot with a fresh OK
    // acknowledgment; delivering it completes the close.
    let delivery =
        stub::deliver_pending_callback(&mut test_case.app, &controller, op_tag::CLOSE_LEASE);
    delivery.assert_event(&completion_event(&lease));
}

#[test]
fn late_close_ack_after_closed_absorbed() {
    let mut test_case = super::create_test_case::<PaymentCurrency>();
    let controller = test_case.address_book.remote_lease_controller().clone();

    let (lease, finish_response) = open_drain_and_finish(&mut test_case);
    finish_response.assert_event(&completion_event(&lease));

    assert_closed_terminal(&mut test_case, &controller, &lease);
}

#[test]
fn close_callback_from_stranger_rejected() {
    let mut test_case = super::create_test_case::<PaymentCurrency>();
    let controller = test_case.address_book.remote_lease_controller().clone();
    stub::set_response_mode(
        &mut test_case.app,
        &controller,
        op_tag::CLOSE_LEASE,
        ResponseMode::Delayed,
    );

    let (lease, _finish_response) = open_drain_and_finish(&mut test_case);

    let err = test_case
        .app
        .execute(
            testing::user(USER),
            lease.clone(),
            &ExecuteMsg::RemoteLeaseCallback(close_lease_ack()),
            &[],
        )
        .expect_err("a stranger's close callback must be rejected");

    let contract_err = err
        .downcast_ref::<ContractError>()
        .expect("must surface as lease ContractError");
    assert!(
        matches!(
            contract_err,
            ContractError::Unauthorized(AccessError::Unauthorized {})
        ),
        "expected ContractError::Unauthorized, got {contract_err:?}"
    );
    assert_eq!(
        1,
        stub::recorded_closes(&test_case.app, &controller, &lease)
    );

    // The rejection left the close in flight: the withheld acknowledgment
    // still completes it.
    let delivery =
        stub::deliver_pending_callback(&mut test_case.app, &controller, op_tag::CLOSE_LEASE);
    delivery.assert_event(&completion_event(&lease));
}

#[test]
fn stale_alarms_ignored_while_closing() {
    let mut test_case = super::create_test_case::<PaymentCurrency>();
    let controller = test_case.address_book.remote_lease_controller().clone();
    stub::set_response_mode(
        &mut test_case.app,
        &controller,
        op_tag::CLOSE_LEASE,
        ResponseMode::Delayed,
    );

    let (lease, _finish_response) = open_drain_and_finish(&mut test_case);

    let time_alarm_response = repay::deliver_funds_arrival_alarm(&mut test_case, lease.clone());
    assert_no_remote_close_events(&time_alarm_response);

    let price_alarm_response = deliver_price_alarm(&mut test_case, lease.clone());
    assert_no_remote_close_events(&price_alarm_response);

    assert_eq!(
        1,
        stub::recorded_closes(&test_case.app, &controller, &lease)
    );

    // The alarms left the close in flight: the withheld acknowledgment
    // still completes it.
    let delivery =
        stub::deliver_pending_callback(&mut test_case.app, &controller, op_tag::CLOSE_LEASE);
    delivery.assert_event(&completion_event(&lease));
}

/// Drive open → full repay → drained-funds arrival → the funds-arrival
/// poll whose response is the finish transaction: customer payout,
/// `CloseLease` emission (per the configured `ResponseMode`), leaser
/// finalize. Asserts the customer payout landed.
///
/// The open/repay and arrival-settlement bodies are adapted from the
/// private drivers in `remote_lease_transfer_out.rs`.
fn open_drain_and_finish(test_case: &mut LeaseTestCase) -> (Addr, AppResponse) {
    let customer = testing::user(USER);
    let (lease, expected_funds, _repay_response) = super::open_and_repay_fully(test_case);
    super::settle_arrival(test_case, &lease, expected_funds);

    let balance_before: LeaseCoin =
        platform::bank::balance(&customer, test_case.app.query()).unwrap();
    let finish_response = repay::deliver_funds_arrival_alarm(test_case, lease.clone());
    assert_eq!(
        balance_before + expected_funds,
        platform::bank::balance(&customer, test_case.app.query()).unwrap()
    );
    (lease, finish_response)
}

/// Prove the lease reached the `Closed` terminal: only the terminal
/// answers a repeated `CloseLease` acknowledgment with the late-ack
/// absorber event — `ClosingRemoteLease` would treat it as completion.
fn assert_closed_terminal(test_case: &mut LeaseTestCase, controller: &Addr, lease: &Addr) {
    let late = stub::inject_callback(&mut test_case.app, controller, lease, close_lease_ack());
    late.assert_event(
        &Event::new(LATE_ACK_EVENT)
            .add_attribute("id", lease.clone())
            .add_attribute("state", "closed"),
    );
    assert_eq!(
        StateResponse::Closed(),
        super::state_query(test_case, lease.clone())
    );
}

fn assert_no_remote_close_events(response: &AppResponse) {
    assert!(
        response
            .events
            .iter()
            .all(|event| event.ty != CLOSING_REMOTE_LEASE_EVENT),
        "expected no {CLOSING_REMOTE_LEASE_EVENT} events, got {:?}",
        response.events
    );
}

fn completion_event(lease: &Addr) -> Event {
    Event::new(CLOSING_REMOTE_LEASE_EVENT)
        .add_attribute("id", lease.clone())
        .add_attribute("remote-lease", "closed")
}

fn close_lease_ack() -> RemoteLeaseCallback {
    RemoteLeaseCallback {
        nonce: 0,
        outcome: RemoteOperationOutcome::OperationOk(WireOperationResponse::CloseLease(
            CloseLeaseResponse {},
        )),
    }
}

fn deliver_price_alarm(test_case: &mut LeaseTestCase, lease: Addr) -> AppResponse {
    let oracle = test_case.address_book.oracle().clone();
    test_case
        .app
        .execute(oracle, lease, &ExecuteMsg::PriceAlarm(), &[])
        .unwrap()
        .unwrap_response()
}
