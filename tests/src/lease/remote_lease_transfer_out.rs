//! Drain-home (Solana → Nolus) E2E (nolus-protocol/ibc-solray#142,
//! transfer-out leg).
//!
//! `Operation::TransferOut` is single-coin per call and the transfers go
//! out strictly sequentially, one in-flight at a time — the 2026-05-27
//! one-in-flight amendment supersedes this file's original "N calls in
//! one batch" sketch. A paid lease's remote holding is exactly its
//! position amount — one currency — so the lease-level drivers exercise
//! the single-coin reality; N-coin sequencing is pinned by the dex
//! package's `RemoteTransferOut` unit tests.
//!
//! The acknowledgment only proves the remote side initiated the
//! transfer; the funds land separately over the paired ICS-20 channel,
//! so the workflow completes through a local funds-arrival poll.
//!
//! Documented drivers:
//!
//! - `transfer_out_single_coin_drain_acks` — the final repay emits one
//!   `ExecuteMsg::TransferOut` carrying the position amount; the inline
//!   acknowledgment advances the lease to the funds-arrival poll; the
//!   arrival alarm completes the close.
//! - `transfer_out_delayed_ack_in_flight_visible` — with
//!   `ResponseMode::Delayed` the in-flight transfer stays observable via
//!   the query until the acknowledgment is delivered.
//! - `transfer_out_error_ack_absorbed_until_heal` — an `OperationErr`
//!   acknowledgment is absorbed with a distinct event and deliberately
//!   NOT auto-retried (a transfer error is plausibly persistent and an
//!   error-bound re-emission has no packet-lifetime cadence);
//!   `ExecuteMsg::Heal` re-emits the in-flight transfer verbatim and the
//!   close completes.
//! - `late_ack_on_closed_absorbed` / `late_ack_from_stranger_rejected` —
//!   the `Closed` terminal absorbs a late `TransferOut` acknowledgment
//!   from the controller pinned at lease open (late-ack event, no state
//!   change) and rejects any other sender.
//! - `drain_callback_from_stranger_rejected` — the in-flight drain
//!   authorises callbacks against the same pinned controller; a stranger
//!   is rejected and the in-flight transfer stays put.

use access_control::error::Error as AccessError;
use currencies::PaymentGroup;
use dex::Error as DexError;
use finance::{coin::CoinDTO, price};
use lease::{
    api::{
        ExecuteMsg,
        query::{StateResponse, paid::ClosingTrx},
    },
    error::ContractError,
};
use platform::coin_legacy;
use remote_lease::{
    callback::{RemoteErrorMessage, RemoteLeaseCallback},
    response::{TransferOutResponse, WireOperationResponse},
};
use sdk::{
    cosmwasm_std::{Addr, Event},
    cw_multi_test::AppResponse,
    testing,
};

use crate::common::{
    self, ADMIN, USER,
    remote_lease_controller_stub::{self as stub, ResponseMode, op_tag},
    test_case::TestCase,
};

use super::{DOWNPAYMENT, LeaseCoin, LeaseTestCase, PaymentCoin, PaymentCurrency, repay};

const CLOSING_TRANSFER_OUT_EVENT: &str = "wasm-ls-close-transfer-out";
const LATE_ACK_EVENT: &str = "wasm-ls-remote-lease-late-ack";

#[test]
fn transfer_out_single_coin_drain_acks() {
    let mut test_case = super::create_test_case::<PaymentCurrency>();
    let controller = test_case.address_book.remote_lease_controller().clone();

    let (lease, expected_funds, repay_response) = open_and_repay_fully(&mut test_case);

    let transfer_outs = stub::recorded_transfer_outs(&test_case.app, &controller, &lease);
    assert_eq!(1, transfer_outs.len());
    assert_eq!(
        &CoinDTO::<PaymentGroup>::from(expected_funds),
        transfer_outs[0].amount()
    );

    repay_response.assert_event(
        &Event::new(CLOSING_TRANSFER_OUT_EVENT).add_attribute("stage", "funds-arrival"),
    );
    assert_closing(
        &test_case,
        &lease,
        expected_funds,
        ClosingTrx::TransferInFinish,
    );

    settle_arrival(&mut test_case, &lease, expected_funds);
    let _arrival = repay::deliver_funds_arrival_alarm(&mut test_case, lease.clone());
    assert_eq!(
        StateResponse::Closed(),
        super::state_query(&test_case, lease)
    );
}

#[test]
fn transfer_out_delayed_ack_in_flight_visible() {
    let mut test_case = super::create_test_case::<PaymentCurrency>();
    let controller = test_case.address_book.remote_lease_controller().clone();
    stub::set_response_mode(
        &mut test_case.app,
        &controller,
        op_tag::TRANSFER_OUT,
        ResponseMode::Delayed,
    );

    let (lease, expected_funds, _repay_response) = open_and_repay_fully(&mut test_case);

    assert_eq!(
        1,
        stub::recorded_transfer_outs(&test_case.app, &controller, &lease).len()
    );
    assert_closing(&test_case, &lease, expected_funds, ClosingTrx::TransferOut);

    let _delivery =
        stub::deliver_pending_callback(&mut test_case.app, &controller, op_tag::TRANSFER_OUT);
    assert_closing(
        &test_case,
        &lease,
        expected_funds,
        ClosingTrx::TransferInFinish,
    );

    settle_arrival(&mut test_case, &lease, expected_funds);
    let _arrival = repay::deliver_funds_arrival_alarm(&mut test_case, lease.clone());
    assert_eq!(
        StateResponse::Closed(),
        super::state_query(&test_case, lease)
    );
}

#[test]
fn transfer_out_error_ack_absorbed_until_heal() {
    let mut test_case = super::create_test_case::<PaymentCurrency>();
    let controller = test_case.address_book.remote_lease_controller().clone();
    let reason = RemoteErrorMessage::new("partial drain failure").expect("within length cap");
    stub::set_response_mode(
        &mut test_case.app,
        &controller,
        op_tag::TRANSFER_OUT,
        ResponseMode::Err(reason),
    );

    let (lease, expected_funds, repay_response) = open_and_repay_fully(&mut test_case);

    repay_response.assert_event(
        &Event::new(CLOSING_TRANSFER_OUT_EVENT).add_attribute("absorbed", "remote-error"),
    );
    assert_eq!(
        1,
        stub::recorded_transfer_outs(&test_case.app, &controller, &lease).len()
    );
    assert_closing(&test_case, &lease, expected_funds, ClosingTrx::TransferOut);

    stub::set_response_mode(
        &mut test_case.app,
        &controller,
        op_tag::TRANSFER_OUT,
        ResponseMode::Ok,
    );
    let heal_response = heal(&mut test_case, lease.clone());
    heal_response
        .assert_event(&Event::new(CLOSING_TRANSFER_OUT_EVENT).add_attribute("heal", "re-emit"));
    assert_eq!(
        2,
        stub::recorded_transfer_outs(&test_case.app, &controller, &lease).len()
    );
    assert_closing(
        &test_case,
        &lease,
        expected_funds,
        ClosingTrx::TransferInFinish,
    );

    settle_arrival(&mut test_case, &lease, expected_funds);
    let _arrival = repay::deliver_funds_arrival_alarm(&mut test_case, lease.clone());
    assert_eq!(
        StateResponse::Closed(),
        super::state_query(&test_case, lease)
    );
}

#[test]
fn drain_callback_from_stranger_rejected() {
    let mut test_case = super::create_test_case::<PaymentCurrency>();
    let controller = test_case.address_book.remote_lease_controller().clone();
    stub::set_response_mode(
        &mut test_case.app,
        &controller,
        op_tag::TRANSFER_OUT,
        ResponseMode::Delayed,
    );

    let (lease, expected_funds, _repay_response) = open_and_repay_fully(&mut test_case);
    assert_closing(&test_case, &lease, expected_funds, ClosingTrx::TransferOut);

    let err = test_case
        .app
        .execute(
            testing::user(USER),
            lease.clone(),
            &ExecuteMsg::RemoteLeaseCallback(transfer_out_ack()),
            &[],
        )
        .expect_err("a stranger's drain callback must be rejected");

    let contract_err = err
        .downcast_ref::<ContractError>()
        .expect("must surface as lease ContractError");
    assert!(
        matches!(
            contract_err,
            ContractError::DexError(DexError::Unauthorized(AccessError::Unauthorized {}))
        ),
        "expected DexError::Unauthorized, got {contract_err:?}"
    );
    assert_closing(&test_case, &lease, expected_funds, ClosingTrx::TransferOut);
}

#[test]
fn late_ack_on_closed_absorbed() {
    let mut test_case = super::create_test_case::<PaymentCurrency>();
    let controller = test_case.address_book.remote_lease_controller().clone();

    let lease = open_and_close(&mut test_case);

    let late = test_case
        .app
        .execute(
            controller,
            lease.clone(),
            &ExecuteMsg::RemoteLeaseCallback(transfer_out_ack()),
            &[],
        )
        .expect("late ack must be absorbed by the Closed terminal")
        .unwrap_response();
    late.assert_event(
        &Event::new(LATE_ACK_EVENT)
            .add_attribute("id", lease.clone())
            .add_attribute("terminal", "closed"),
    );
    assert_eq!(
        StateResponse::Closed(),
        super::state_query(&test_case, lease)
    );
}

#[test]
fn late_ack_from_stranger_rejected() {
    let mut test_case = super::create_test_case::<PaymentCurrency>();

    let lease = open_and_close(&mut test_case);

    let err = test_case
        .app
        .execute(
            testing::user(USER),
            lease.clone(),
            &ExecuteMsg::RemoteLeaseCallback(transfer_out_ack()),
            &[],
        )
        .expect_err("a stranger's late ack must be rejected");

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
        StateResponse::Closed(),
        super::state_query(&test_case, lease)
    );
}

/// Open a lease and repay the whole loan, leaving the drain started with
/// whatever `ResponseMode` the test configured for `transfer_out`
fn open_and_repay_fully(test_case: &mut LeaseTestCase) -> (Addr, LeaseCoin, AppResponse) {
    let downpayment = DOWNPAYMENT;
    let lease = super::open_lease(test_case, downpayment, None);

    let borrowed_lpn = super::quote_borrow(test_case, downpayment);
    let borrowed: PaymentCoin =
        price::total(borrowed_lpn, super::price_lpn_of::<PaymentCurrency>().inv()).unwrap();
    let expected_funds: LeaseCoin = super::expected_opened_amount(downpayment, borrowed_lpn);

    let repay_response =
        repay::repay_with_hook_on_swap(test_case, lease.clone(), borrowed, |_app| {})
            .unwrap_response();
    (lease, expected_funds, repay_response)
}

fn assert_closing(
    test_case: &LeaseTestCase,
    lease: &Addr,
    expected_funds: LeaseCoin,
    in_progress: ClosingTrx,
) {
    assert_eq!(
        StateResponse::Closing {
            amount: expected_funds.into(),
            in_progress,
        },
        super::state_query(test_case, lease.clone())
    );
}

/// Mirror the acknowledged transfer onto the bank balances: the remote
/// account (stood in by the ICA address) escrows the asset and the paired
/// ICS-20 channel lands it on the lease's local account
fn settle_arrival(test_case: &mut LeaseTestCase, lease: &Addr, funds: LeaseCoin) {
    let ica_addr: Addr = TestCase::ica_addr(lease, TestCase::LEASE_ICA_ID);
    test_case
        .app
        .send_tokens(
            ica_addr,
            testing::user(ADMIN),
            &[coin_legacy::to_cosmwasm_on_dex(funds)],
        )
        .unwrap();
    test_case
        .app
        .send_tokens(
            testing::user(ADMIN),
            lease.clone(),
            &[common::cwcoin(funds)],
        )
        .unwrap();
}

/// Drive a lease through the full drain to the `Closed` terminal
fn open_and_close(test_case: &mut LeaseTestCase) -> Addr {
    let (lease, expected_funds, _repay_response) = open_and_repay_fully(test_case);
    settle_arrival(test_case, &lease, expected_funds);
    let _arrival = repay::deliver_funds_arrival_alarm(test_case, lease.clone());
    assert_eq!(
        StateResponse::Closed(),
        super::state_query(test_case, lease.clone())
    );
    lease
}

fn transfer_out_ack() -> RemoteLeaseCallback {
    RemoteLeaseCallback::OperationOk(WireOperationResponse::TransferOut(TransferOutResponse {}))
}

fn heal(test_case: &mut LeaseTestCase, lease: Addr) -> AppResponse {
    test_case
        .app
        .execute(testing::user(USER), lease, &ExecuteMsg::Heal(), &[])
        .unwrap()
        .unwrap_response()
}
