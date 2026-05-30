//! Close-lifecycle E2E (issue #142 Phase 6).
//!
//! ⚠️ UNREGISTERED ⚠️ — targets the **post-refactor** lease state machine
//! and will not compile on `main`. Registration in
//! `tests/src/lease/mod.rs` is intentionally **omitted**; Phase 6
//! un-comments the registration once the new `CloseLease` state lands.
//! See `.claude/handoffs/2026-05-25-issue142-plan.md`, §5 Phase 6 and
//! §10.C.5 (divergence terminal absorber).
//!
//! Acceptance criteria (plan §5 Phase 2 and §4 Q5):
//!
//! - `close_lifecycle_happy_path` — after the drain-home TransferOut
//!   acks all clear and per-currency residuals are zero
//!   (`invariant_held()`), the lease emits one
//!   `ExecuteMsg::CloseLease`; on `OperationOk(CloseLease(_))`, the
//!   lease transitions to the `Closed` terminal.
//! - `close_divergence_on_error_emits_event_and_stays_queryable` —
//!   `OperationErr` on `CloseLease` represents Cosmos↔Solana state
//!   divergence (§4 Q5). Emits `wasm-remote-lease-divergence`, moves
//!   to the divergence terminal, lease remains queryable for operator
//!   intervention. Does NOT auto-retry.
//! - `close_timeout_does_retry_via_time_alarm` — `OperationTimeout`
//!   on `CloseLease` IS retried (transport fault, not divergence).
//! - `late_close_ack_after_divergence_is_absorbed` — UNORDERED-channel
//!   stale-ack absorber on the divergence terminal (§10.C.5): a late
//!   OK ack returns `Ok` + emits `wasm-remote-lease-late-ack`.

use lease::api::query::StateResponse;
use remote_lease::{
    callback::{RemoteErrorMessage, RemoteLeaseCallback},
    response::OperationResponse,
};
use sdk::cosmwasm_std::Addr;

use crate::common::remote_lease_controller_stub::{self as stub, ResponseMode, op_tag};

use super::{LeaseCurrency, LeaseTestCase};

#[test]
fn close_lifecycle_happy_path() {
    let _test_case = super::create_test_case::<LeaseCurrency>();
    panic!("Phase 6 must implement CloseLease happy-path driver");
}

#[test]
fn close_divergence_on_error_emits_event_and_stays_queryable() {
    let mut test_case = super::create_test_case::<LeaseCurrency>();
    let controller = test_case.address_book.remote_lease_controller().clone();
    let reason = RemoteErrorMessage::new("balance mismatch with solana state")
        .expect("within length cap");
    stub::set_response_mode(
        &mut test_case.app,
        &controller,
        op_tag::CLOSE_LEASE,
        ResponseMode::Err(reason),
    );

    panic!("Phase 6 must implement divergence-terminal assertion + event check");
}

#[test]
fn close_timeout_does_retry_via_time_alarm() {
    let _test_case = super::create_test_case::<LeaseCurrency>();
    panic!("Phase 6 must implement CloseLease timeout-retry driver");
}

#[test]
fn late_close_ack_after_divergence_is_absorbed() {
    let mut test_case = super::create_test_case::<LeaseCurrency>();
    let controller = test_case.address_book.remote_lease_controller().clone();
    stub::set_response_mode(
        &mut test_case.app,
        &controller,
        op_tag::CLOSE_LEASE,
        ResponseMode::Delayed,
    );

    panic!("Phase 6 must implement late-ack absorber on divergence terminal");
}

// Silence "unused import" warnings while the module is unregistered.
#[allow(dead_code)]
fn _unused_imports_anchor(
    _: StateResponse,
    _: RemoteLeaseCallback,
    _: OperationResponse,
    _: Addr,
    _: LeaseTestCase,
) {
}
