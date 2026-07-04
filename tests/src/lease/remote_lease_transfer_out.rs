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
//! - `transfer_out_heal_supersedes_original_ack` — a heal re-emits the
//!   in-flight transfer with a fresh nonce; the original packet's late
//!   acknowledgment (the superseded nonce) is then absorbed as
//!   `nonce-mismatch` instead of advancing the drain, and the healed
//!   re-emission's acknowledgment completes the close.
//! - `late_ack_on_closed_absorbed` / `late_ack_from_stranger_rejected` —
//!   the `Closed` terminal absorbs a late `TransferOut` acknowledgment
//!   from the controller pinned at lease open (late-ack event, no state
//!   change) and rejects any other sender.
//! - `drain_callback_from_stranger_rejected` — the in-flight drain
//!   authorises callbacks against the same pinned controller; a stranger
//!   is rejected and the in-flight transfer stays put.
//!
//! Absorb-arm coverage (the behaviour already ships):
//!
//! - `funds_arrival_absorbs_remote_error_callback` /
//!   `funds_arrival_absorbs_remote_timeout_callback` /
//!   `funds_arrival_absorbs_stray_ok_callback` — once every transfer has
//!   acknowledged the drain sits at `FundsArrival` (`ClosingTrx::TransferInFinish`),
//!   which scheduled no remote operation; an error, a timeout, or a stray
//!   success callback delivered now is absorbed with the generic
//!   `wasm-remote-callback` event (`absorbed = error|timeout|response`), the
//!   controller's ack commits, and the stage is unchanged.
//! - `transfer_out_wrong_variant_ack_absorbed` — a decodable success ack for a
//!   non-transfer-out operation, carrying the in-flight nonce, is rejected by
//!   the transfer decode and absorbed (`unexpected-response-variant`).
//! - `transfer_out_undecodable_ack_absorbed` — a success ack whose swap payload
//!   names a ticker outside the registry fails the typed decode and is absorbed
//!   (`undecodable-response`).
//! - `transfer_out_stale_nonce_error_absorbed` /
//!   `transfer_out_stale_nonce_timeout_absorbed` — an error or a timeout callback
//!   carrying a superseded nonce is absorbed (`nonce-mismatch`) before the
//!   error/timeout arm runs, leaving the in-flight transfer put.

use crate::common::testing;
use access_control::error::Error as AccessError;
use currencies::PaymentGroup;
use dex::Error as DexError;
use finance::coin::CoinDTO;
use lease::{
    api::{
        ExecuteMsg,
        query::{StateResponse, paid::ClosingTrx},
    },
    error::ContractError,
};
use remote_lease::{
    callback::{RemoteErrorMessage, RemoteLeaseCallback, RemoteOperationOutcome},
    response::{
        CloseLeaseResponse, Ticker, TransferOutResponse, WireCoin, WireOperationResponse,
        WireSwapResponse,
    },
};
use sdk::cosmwasm_std::{Addr, Event};

use crate::common::{
    USER,
    remote_lease_controller_stub::{self as stub, ResponseMode, op_tag},
};

use super::{LeaseCoin, LeaseTestCase, PaymentCurrency, repay};

const CLOSING_TRANSFER_OUT_EVENT: &str = "wasm-ls-close-transfer-out";
const LATE_ACK_EVENT: &str = "wasm-ls-remote-lease-late-ack";
/// The generic absorb event the drain's `FundsArrival` stage emits through the
/// `Handler` defaults for any callback that reaches it — the arrival stage
/// scheduled no remote operation, so every kind is absorbed with this event.
const REMOTE_CALLBACK_ABSORB_EVENT: &str = "wasm-remote-callback";
/// A nonce below the close-leg drain's first-emission nonce (`1`), so a callback
/// stamped with it can never match the in-flight transfer and is absorbed as a
/// stale duplicate.
const STALE_NONCE: u64 = 0;

#[test]
fn transfer_out_single_coin_drain_acks() {
    let mut test_case = super::create_test_case::<PaymentCurrency>();
    let controller = test_case.address_book.remote_lease_controller().clone();

    let (lease, expected_funds, repay_response) = super::open_and_repay_fully(&mut test_case);

    // Two transfer-outs ride the controller: the repay-proceeds drain and,
    // once the loan is paid, the close leg. The close leg is the last one
    // and carries the freed lease asset.
    let transfer_outs = stub::recorded_transfer_outs(&test_case.app, &controller, &lease);
    assert_eq!(2, transfer_outs.len());
    assert_eq!(
        &CoinDTO::<PaymentGroup>::from(expected_funds),
        transfer_outs[1].amount()
    );

    repay_response.assert_event(
        &Event::new(CLOSING_TRANSFER_OUT_EVENT).add_attribute("stage", "funds-arrival"),
    );
    assert_closing(
        expected_funds,
        ClosingTrx::TransferInFinish,
        &test_case,
        &lease,
    );

    super::settle_arrival(&mut test_case, &lease, expected_funds);
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

    // Delay only the close-leg transfer-out: the repay-proceeds drain has
    // already acked when the hook runs, so the mode applies solely to the
    // close leg.
    let (lease, expected_funds, _repay_response) = {
        let controller = controller.clone();
        super::open_and_repay_fully_then(&mut test_case, move |app| {
            stub::set_response_mode(
                app,
                &controller,
                op_tag::TRANSFER_OUT,
                ResponseMode::Delayed,
            );
        })
    };

    // The repay drain transferred out and acked; the close leg is the
    // second transfer-out and is the one held in flight.
    assert_eq!(
        2,
        stub::recorded_transfer_outs(&test_case.app, &controller, &lease).len()
    );
    assert_closing(expected_funds, ClosingTrx::TransferOut, &test_case, &lease);

    let _delivery =
        stub::deliver_pending_callback(&mut test_case.app, &controller, op_tag::TRANSFER_OUT);
    assert_closing(
        expected_funds,
        ClosingTrx::TransferInFinish,
        &test_case,
        &lease,
    );

    super::settle_arrival(&mut test_case, &lease, expected_funds);
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

    // Error only the close-leg transfer-out: the repay-proceeds drain has
    // already acked when the hook runs.
    let (lease, expected_funds, repay_response) = {
        let controller = controller.clone();
        super::open_and_repay_fully_then(&mut test_case, move |app| {
            stub::set_response_mode(
                app,
                &controller,
                op_tag::TRANSFER_OUT,
                ResponseMode::Err(reason),
            );
        })
    };

    repay_response.assert_event(
        &Event::new(CLOSING_TRANSFER_OUT_EVENT).add_attribute("absorbed", "remote-error"),
    );
    // The repay drain transfer-out plus the errored close attempt (recorded
    // even on error) make two; the heal re-emit below adds the third.
    assert_eq!(
        2,
        stub::recorded_transfer_outs(&test_case.app, &controller, &lease).len()
    );
    assert_closing(expected_funds, ClosingTrx::TransferOut, &test_case, &lease);

    stub::set_response_mode(
        &mut test_case.app,
        &controller,
        op_tag::TRANSFER_OUT,
        ResponseMode::Ok,
    );
    let heal_response = super::heal(&mut test_case, lease.clone());
    heal_response
        .assert_event(&Event::new(CLOSING_TRANSFER_OUT_EVENT).add_attribute("heal", "re-emit"));
    assert_eq!(
        3,
        stub::recorded_transfer_outs(&test_case.app, &controller, &lease).len()
    );
    assert_closing(
        expected_funds,
        ClosingTrx::TransferInFinish,
        &test_case,
        &lease,
    );

    super::settle_arrival(&mut test_case, &lease, expected_funds);
    let _arrival = repay::deliver_funds_arrival_alarm(&mut test_case, lease.clone());
    assert_eq!(
        StateResponse::Closed(),
        super::state_query(&test_case, lease)
    );
}

// A heal bumps the in-flight nonce, so the original packet's late acknowledgment
// no longer matches and is absorbed instead of advancing the drain; the healed
// re-emission's ack then completes the close. End-to-end proof that the
// duplicate-acknowledgment window is closed.
#[test]
fn transfer_out_heal_supersedes_original_ack() {
    // The close-leg drain is a fresh single-coin transfer-out, so its first
    // emission carries nonce 1; the heal below re-emits with nonce 2.
    const ORIGINAL_NONCE: u64 = 1;

    let mut test_case = super::create_test_case::<PaymentCurrency>();
    let controller = test_case.address_book.remote_lease_controller().clone();

    // Hold only the close-leg transfer-out in flight: the repay-proceeds drain
    // has already acked when the hook runs.
    let (lease, expected_funds, _repay_response) = {
        let controller = controller.clone();
        super::open_and_repay_fully_then(&mut test_case, move |app| {
            stub::set_response_mode(
                app,
                &controller,
                op_tag::TRANSFER_OUT,
                ResponseMode::Delayed,
            );
        })
    };
    assert_closing(expected_funds, ClosingTrx::TransferOut, &test_case, &lease);

    // The operator heals the still-in-flight transfer, re-emitting it with a
    // fresh nonce that supersedes the original packet.
    let heal_response = super::heal(&mut test_case, lease.clone());
    heal_response
        .assert_event(&Event::new(CLOSING_TRANSFER_OUT_EVENT).add_attribute("heal", "re-emit"));

    // The original packet's late ack carries the now-superseded nonce: it is
    // absorbed, not credited, and the drain stays in transfer-out.
    let stale = stub::inject_callback_with_nonce(
        &mut test_case.app,
        &controller,
        &lease,
        ORIGINAL_NONCE,
        RemoteOperationOutcome::OperationOk(WireOperationResponse::TransferOut(
            TransferOutResponse {},
        )),
    );
    stale.assert_event(
        &Event::new(CLOSING_TRANSFER_OUT_EVENT).add_attribute("absorbed", "nonce-mismatch"),
    );
    assert_closing(expected_funds, ClosingTrx::TransferOut, &test_case, &lease);

    // The healed re-emission's ack (the fresh nonce the stand-in holds pending)
    // matches the in-flight transfer and completes the drain.
    let _delivery =
        stub::deliver_pending_callback(&mut test_case.app, &controller, op_tag::TRANSFER_OUT);
    assert_closing(
        expected_funds,
        ClosingTrx::TransferInFinish,
        &test_case,
        &lease,
    );

    super::settle_arrival(&mut test_case, &lease, expected_funds);
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

    // Hold only the close-leg transfer-out in flight: the repay-proceeds
    // drain has already acked when the hook runs.
    let (lease, expected_funds, _repay_response) = {
        let controller = controller.clone();
        super::open_and_repay_fully_then(&mut test_case, move |app| {
            stub::set_response_mode(
                app,
                &controller,
                op_tag::TRANSFER_OUT,
                ResponseMode::Delayed,
            );
        })
    };
    assert_closing(expected_funds, ClosingTrx::TransferOut, &test_case, &lease);

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
    assert_closing(expected_funds, ClosingTrx::TransferOut, &test_case, &lease);
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
            .add_attribute("state", "closed"),
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

#[test]
fn funds_arrival_absorbs_remote_error_callback() {
    let mut test_case = super::create_test_case::<PaymentCurrency>();
    let controller = test_case.address_book.remote_lease_controller().clone();

    // The default `Ok` mode acks the close transfer-out inline, so the drain
    // already sits at the funds-arrival poll.
    let (lease, expected_funds, _repay_response) = super::open_and_repay_fully(&mut test_case);
    assert_closing(
        expected_funds,
        ClosingTrx::TransferInFinish,
        &test_case,
        &lease,
    );

    let reason = RemoteErrorMessage::new("late error at arrival").expect("within length cap");
    let absorbed = stub::inject_callback(
        &mut test_case.app,
        &controller,
        &lease,
        RemoteLeaseCallback {
            nonce: 0,
            outcome: RemoteOperationOutcome::OperationErr(reason),
        },
    );
    absorbed
        .assert_event(&Event::new(REMOTE_CALLBACK_ABSORB_EVENT).add_attribute("absorbed", "error"));
    assert_closing(
        expected_funds,
        ClosingTrx::TransferInFinish,
        &test_case,
        &lease,
    );
}

#[test]
fn funds_arrival_absorbs_remote_timeout_callback() {
    let mut test_case = super::create_test_case::<PaymentCurrency>();
    let controller = test_case.address_book.remote_lease_controller().clone();

    let (lease, expected_funds, _repay_response) = super::open_and_repay_fully(&mut test_case);
    assert_closing(
        expected_funds,
        ClosingTrx::TransferInFinish,
        &test_case,
        &lease,
    );

    let absorbed = stub::inject_callback(
        &mut test_case.app,
        &controller,
        &lease,
        RemoteLeaseCallback {
            nonce: 0,
            outcome: RemoteOperationOutcome::OperationTimeout,
        },
    );
    absorbed.assert_event(
        &Event::new(REMOTE_CALLBACK_ABSORB_EVENT).add_attribute("absorbed", "timeout"),
    );
    assert_closing(
        expected_funds,
        ClosingTrx::TransferInFinish,
        &test_case,
        &lease,
    );
}

#[test]
fn funds_arrival_absorbs_stray_ok_callback() {
    let mut test_case = super::create_test_case::<PaymentCurrency>();
    let controller = test_case.address_book.remote_lease_controller().clone();

    let (lease, expected_funds, _repay_response) = super::open_and_repay_fully(&mut test_case);
    assert_closing(
        expected_funds,
        ClosingTrx::TransferInFinish,
        &test_case,
        &lease,
    );

    let absorbed = stub::inject_callback(
        &mut test_case.app,
        &controller,
        &lease,
        RemoteLeaseCallback {
            nonce: 0,
            outcome: RemoteOperationOutcome::OperationOk(WireOperationResponse::TransferOut(
                TransferOutResponse {},
            )),
        },
    );
    absorbed.assert_event(
        &Event::new(REMOTE_CALLBACK_ABSORB_EVENT).add_attribute("absorbed", "response"),
    );
    assert_closing(
        expected_funds,
        ClosingTrx::TransferInFinish,
        &test_case,
        &lease,
    );
}

#[test]
fn transfer_out_wrong_variant_ack_absorbed() {
    let mut test_case = super::create_test_case::<PaymentCurrency>();
    let controller = test_case.address_book.remote_lease_controller().clone();

    let (lease, expected_funds, _repay_response) = {
        let controller = controller.clone();
        super::open_and_repay_fully_then(&mut test_case, move |app| {
            stub::set_response_mode(
                app,
                &controller,
                op_tag::TRANSFER_OUT,
                ResponseMode::Delayed,
            );
        })
    };
    assert_closing(expected_funds, ClosingTrx::TransferOut, &test_case, &lease);

    // The injected callback is stamped with the in-flight transfer's nonce, so
    // it clears the nonce gate and reaches the transfer decode, which rejects a
    // decodable non-transfer-out response as the wrong variant.
    let absorbed = stub::inject_callback(
        &mut test_case.app,
        &controller,
        &lease,
        RemoteLeaseCallback {
            nonce: 0,
            outcome: RemoteOperationOutcome::OperationOk(WireOperationResponse::CloseLease(
                CloseLeaseResponse {},
            )),
        },
    );
    absorbed.assert_event(
        &Event::new(CLOSING_TRANSFER_OUT_EVENT)
            .add_attribute("absorbed", "unexpected-response-variant"),
    );
    assert_closing(expected_funds, ClosingTrx::TransferOut, &test_case, &lease);
}

#[test]
fn transfer_out_undecodable_ack_absorbed() {
    let mut test_case = super::create_test_case::<PaymentCurrency>();
    let controller = test_case.address_book.remote_lease_controller().clone();

    let (lease, expected_funds, _repay_response) = {
        let controller = controller.clone();
        super::open_and_repay_fully_then(&mut test_case, move |app| {
            stub::set_response_mode(
                app,
                &controller,
                op_tag::TRANSFER_OUT,
                ResponseMode::Delayed,
            );
        })
    };
    assert_closing(expected_funds, ClosingTrx::TransferOut, &test_case, &lease);

    // A swap payload naming a ticker outside the currency registry passes the
    // controller wire-shaped but fails the lease's typed decode outright — an
    // undecodable payload, distinct from a decodable wrong variant.
    let absorbed = stub::inject_callback(
        &mut test_case.app,
        &controller,
        &lease,
        RemoteLeaseCallback {
            nonce: 0,
            outcome: RemoteOperationOutcome::OperationOk(WireOperationResponse::Swap(
                WireSwapResponse {
                    amount_out: WireCoin::new(42, Ticker::new("NOT_IN_REGISTRY")),
                },
            )),
        },
    );
    absorbed.assert_event(
        &Event::new(CLOSING_TRANSFER_OUT_EVENT).add_attribute("absorbed", "undecodable-response"),
    );
    assert_closing(expected_funds, ClosingTrx::TransferOut, &test_case, &lease);
}

#[test]
fn transfer_out_stale_nonce_error_absorbed() {
    let mut test_case = super::create_test_case::<PaymentCurrency>();
    let controller = test_case.address_book.remote_lease_controller().clone();

    let (lease, expected_funds, _repay_response) = {
        let controller = controller.clone();
        super::open_and_repay_fully_then(&mut test_case, move |app| {
            stub::set_response_mode(
                app,
                &controller,
                op_tag::TRANSFER_OUT,
                ResponseMode::Delayed,
            );
        })
    };
    assert_closing(expected_funds, ClosingTrx::TransferOut, &test_case, &lease);

    // The nonce gate runs before the error arm, so a superseded-nonce error is
    // absorbed as a stale duplicate rather than as a remote error.
    let reason = RemoteErrorMessage::new("stale error ack").expect("within length cap");
    let absorbed = stub::inject_callback_with_nonce(
        &mut test_case.app,
        &controller,
        &lease,
        STALE_NONCE,
        RemoteOperationOutcome::OperationErr(reason),
    );
    absorbed.assert_event(
        &Event::new(CLOSING_TRANSFER_OUT_EVENT).add_attribute("absorbed", "nonce-mismatch"),
    );
    assert_closing(expected_funds, ClosingTrx::TransferOut, &test_case, &lease);
}

#[test]
fn transfer_out_stale_nonce_timeout_absorbed() {
    let mut test_case = super::create_test_case::<PaymentCurrency>();
    let controller = test_case.address_book.remote_lease_controller().clone();

    let (lease, expected_funds, _repay_response) = {
        let controller = controller.clone();
        super::open_and_repay_fully_then(&mut test_case, move |app| {
            stub::set_response_mode(
                app,
                &controller,
                op_tag::TRANSFER_OUT,
                ResponseMode::Delayed,
            );
        })
    };
    assert_closing(expected_funds, ClosingTrx::TransferOut, &test_case, &lease);

    // The nonce gate runs before the timeout arm, so a superseded-nonce timeout
    // is absorbed as a stale duplicate rather than re-emitting the transfer.
    let absorbed = stub::inject_callback_with_nonce(
        &mut test_case.app,
        &controller,
        &lease,
        STALE_NONCE,
        RemoteOperationOutcome::OperationTimeout,
    );
    absorbed.assert_event(
        &Event::new(CLOSING_TRANSFER_OUT_EVENT).add_attribute("absorbed", "nonce-mismatch"),
    );
    assert_closing(expected_funds, ClosingTrx::TransferOut, &test_case, &lease);
}

fn assert_closing(
    expected_funds: LeaseCoin,
    in_progress: ClosingTrx,
    test_case: &LeaseTestCase,
    lease: &Addr,
) {
    assert_eq!(
        StateResponse::Closing {
            amount: expected_funds.into(),
            in_progress,
        },
        super::state_query(test_case, lease.clone())
    );
}

/// Drive a lease through the full drain to the `Closed` terminal
fn open_and_close(test_case: &mut LeaseTestCase) -> Addr {
    let (lease, expected_funds, _repay_response) = super::open_and_repay_fully(test_case);
    super::settle_arrival(test_case, &lease, expected_funds);
    let _arrival = repay::deliver_funds_arrival_alarm(test_case, lease.clone());
    assert_eq!(
        StateResponse::Closed(),
        super::state_query(test_case, lease.clone())
    );
    lease
}

fn transfer_out_ack() -> RemoteLeaseCallback {
    RemoteLeaseCallback {
        nonce: 0,
        outcome: RemoteOperationOutcome::OperationOk(WireOperationResponse::TransferOut(
            TransferOutResponse {},
        )),
    }
}
