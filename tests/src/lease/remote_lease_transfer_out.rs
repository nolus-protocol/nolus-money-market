//! Drain-home (Solana → Nolus) E2E (issue #142 Phase 5).
//!
//! ⚠️ UNREGISTERED ⚠️ — targets the **post-refactor** lease state machine
//! and will not compile on `main`. Registration in
//! `tests/src/lease/mod.rs` is intentionally **omitted**; Phase 5
//! un-comments the registration once the post-refactor `transfer_in_init`
//! / `transfer_in_finish` paths land. See
//! `.claude/handoffs/2026-05-25-issue142-plan.md`, §5 Phase 5,
//! §10.A.6 (TransferOut single-coin per call), and §3.4.
//!
//! Atomicity model (ADR §3.5 / plan §10.A.6):
//!
//! `Operation::TransferOut` is **single-coin per call**. The lease's
//! drain-home state-enter emits N `Lease::transfer_out` calls in one
//! batch (one per held currency); the stand-in synthesises N
//! independent acks. `acks_left = N` counter decrements per ack;
//! transition to `CloseLease` after the Nth.
//!
//! Acceptance criteria (plan §5 Phase 2):
//!
//! - `transfer_out_single_coin_drain_acks` — one held currency → one
//!   `ExecuteMsg::TransferOut` → one ack → transition.
//! - `transfer_out_multi_currency_parallel_batch` — N held currencies
//!   in one drain state-enter → N sequential `ExecuteMsg::TransferOut`
//!   calls in the same batch; per-currency residual decrements as each
//!   ack arrives; the in-flight count visible via the query while N-1
//!   acks remain (`ResponseMode::Delayed` on one of the N).
//! - `transfer_out_partial_failure_keeps_remaining_in_flight` — for the
//!   N-1 successful acks, the lease accepts them; the Nth carries
//!   `OperationErr`, asserting the lease's classifier handles a
//!   partial-batch failure deterministically (does NOT crash, surfaces
//!   the failure for operator intervention).

use lease::api::query::{StateResponse, opened::OngoingTrx as OpenedOngoingTrx};
use remote_lease::{
    callback::{RemoteErrorMessage, RemoteLeaseCallback},
    response::OperationResponse,
};
use sdk::cosmwasm_std::Addr;

use crate::common::remote_lease_controller_stub::{self as stub, ResponseMode, op_tag};

use super::{LeaseCurrency, LeaseTestCase};

#[test]
fn transfer_out_single_coin_drain_acks() {
    let _test_case = super::create_test_case::<LeaseCurrency>();
    panic!("Phase 5 must implement single-coin TransferOut drain driver");
}

#[test]
fn transfer_out_multi_currency_parallel_batch() {
    let mut test_case = super::create_test_case::<LeaseCurrency>();
    let controller = test_case.address_book.remote_lease_controller().clone();
    // Use Delayed for the first TransferOut ack; assert that after the
    // second arrives, the lease still reports the first as in-flight.
    stub::set_response_mode(
        &mut test_case.app,
        &controller,
        op_tag::TRANSFER_OUT,
        ResponseMode::Delayed,
    );

    panic!("Phase 5 must implement N-coin TransferOut batch + in-flight assertion");
}

#[test]
fn transfer_out_partial_failure_keeps_remaining_in_flight() {
    let mut test_case = super::create_test_case::<LeaseCurrency>();
    let controller = test_case.address_book.remote_lease_controller().clone();
    let reason = RemoteErrorMessage::new("partial drain failure")
        .expect("within length cap");
    stub::set_response_mode(
        &mut test_case.app,
        &controller,
        op_tag::TRANSFER_OUT,
        ResponseMode::Err(reason),
    );

    panic!("Phase 5 must implement partial-batch failure driver");
}

// Silence "unused import" warnings while the module is unregistered.
#[allow(dead_code)]
fn _unused_imports_anchor(
    _: StateResponse,
    _: OpenedOngoingTrx,
    _: RemoteLeaseCallback,
    _: OperationResponse,
    _: Addr,
    _: LeaseTestCase,
) {
}
