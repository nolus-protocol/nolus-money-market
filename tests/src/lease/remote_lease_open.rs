//! Open-lifecycle E2E (issue #142 Phase 3).
//!
//! ⚠️ UNREGISTERED ⚠️ — this module targets the **post-refactor** lease
//! state machine and will not compile on `main`. It is committed alongside
//! the Test Architect's Phase 2 work as the failing-test layer for Phase 3.
//! Registration in `tests/src/lease/mod.rs` is intentionally **omitted**;
//! Phase 3 un-comments the `mod remote_lease_open;` line in that file once
//! the post-refactor surface lands. See the plan
//! `.claude/handoffs/2026-05-25-issue142-plan.md`, §5 Phase 2-3, and §10.A.2 /
//! §10.B.2.
//!
//! Targets the post-refactor symbols:
//!
//! - `lease::api::query::opening::OngoingTrx::OpenLease { remote_lease: RemoteLeaseId }`
//!   (renamed from the current `OpenIcaAccount` variant — plan §3.3).
//! - `lease::api::query::Status::OpenFailed { reason: RemoteErrorMessage }`
//!   (new top-level status variant — plan §10.B.2).
//! - The `wasm-remote-lease-open-failed` and `wasm-remote-lease-late-ack`
//!   events emitted from the post-refactor state machine
//!   (plan §10.B.2 / §10.A.2).
//!
//! Acceptance criteria (plan §5 Phase 2):
//!
//! - `open_lifecycle_happy_path` — OpenLease → BuyAsset transition; the
//!   `remote_lease_id` returned by the stand-in is persisted on the lease
//!   and visible via `OngoingTrx::OpenLease { remote_lease }`.
//! - `open_failed_on_error_refunds_and_finalises` — `OperationErr`
//!   triggers atomic LPP repay + customer downpayment refund + leaser
//!   finalise + `Status::OpenFailed { reason }` query + the
//!   `wasm-remote-lease-open-failed` event (§10.B.2).
//! - `late_open_lease_ack_after_open_failed_is_absorbed` — UNORDERED-channel
//!   stale-ack absorber (§10.A.2): after the terminal `OpenFailed` is
//!   reached, a late OK ack returns `Ok` + emits `wasm-remote-lease-late-ack`,
//!   balances unchanged.

use currencies::PaymentGroup;
use lease::{
    api::{
        ExecuteMsg,
        query::{
            StateResponse,
            opening::OngoingTrx as OpeningOngoingTrx,
            // Symbol introduced in Phase 3:
            // Status,
        },
    },
    error::ContractError,
};
use remote_lease::{
    callback::{RemoteErrorMessage, RemoteLeaseCallback},
    response::{OpenLeaseResponse, OperationResponse, RemoteLeaseId},
};
use sdk::{
    cosmwasm_std::Addr,
    cw_multi_test::AppResponse,
    testing,
};

use crate::common::{
    self, USER,
    remote_lease_controller_stub::{self as stub, ResponseMode, op_tag},
    test_case::response::ResponseWithInterChainMsgs,
};

use super::{LeaseCoin, LeaseCurrency, LeaseTestCase};

const DOWNPAYMENT: LeaseCoin = LeaseCoin::new(10_000);

#[test]
fn open_lifecycle_happy_path() {
    let mut test_case = super::create_test_case::<LeaseCurrency>();
    let lease = super::try_init_lease(&mut test_case, DOWNPAYMENT, None);

    // Driving the funds-transfer + OpenLease ack should land us in the
    // BuyAsset state with `remote_lease` set to the stand-in's synthetic
    // PDA. The exact helper signature lands with Phase 3 — for now the
    // test reads `OngoingTrx::OpenLease { remote_lease }` directly.

    let state = super::state_query(&test_case, lease);

    let remote_lease = match state {
        StateResponse::Opening { in_progress, .. } => match in_progress {
            OpeningOngoingTrx::OpenLease { remote_lease } => remote_lease,
            other => panic!("expected OngoingTrx::OpenLease, got {other:?}"),
        },
        other => panic!("expected StateResponse::Opening, got {other:?}"),
    };

    assert!(
        remote_lease.as_str().starts_with("StubPda"),
        "expected stand-in PDA prefix, got {remote_lease:?}",
    );
}

#[test]
fn open_failed_on_error_refunds_and_finalises() {
    let mut test_case = super::create_test_case::<LeaseCurrency>();
    let controller = test_case.address_book.remote_lease_controller().clone();
    let reason = RemoteErrorMessage::new("solana side rejected").expect("within length cap");
    stub::set_response_mode(
        &mut test_case.app,
        &controller,
        op_tag::OPEN_LEASE,
        ResponseMode::Err(reason.clone()),
    );

    // Phase 3 wires the auto-refund batch: LPP repay + downpayment back
    // to USER + leaser finalise + Status::OpenFailed { reason }.
    // The exact driver helper lands with Phase 3. Asserted post-conditions:
    //
    //   - StateResponse exposes Status::OpenFailed { reason }
    //   - balance(USER) increased by DOWNPAYMENT
    //   - leaser reports no live lease for USER
    //   - the AppResponse carries a `wasm-remote-lease-open-failed` event

    let _lease = super::try_init_lease(&mut test_case, DOWNPAYMENT, None);

    // Placeholder — actual assertions hook in with Phase 3. The lease
    // helper (currently `confirm_ica_and_transfer_funds`) must be
    // replaced with a `confirm_open_lease_callback` equivalent.
    panic!(
        "Phase 3 must implement the OperationErr auto-refund driver; \
         reason captured: {reason:?}"
    );
}

#[test]
fn late_open_lease_ack_after_open_failed_is_absorbed() {
    let mut test_case = super::create_test_case::<LeaseCurrency>();
    let controller = test_case.address_book.remote_lease_controller().clone();
    // Configure the stub to first time out the OpenLease packet (auto-
    // refund + transition to OpenFailed). Then dispatch a *late* success
    // ack via the test-only DeliverPending path.
    stub::set_response_mode(
        &mut test_case.app,
        &controller,
        op_tag::OPEN_LEASE,
        ResponseMode::Delayed,
    );

    // Drive the open path through the timeout branch; then deliver the
    // late OK ack and assert:
    //   - app.execute returns Ok (absorber rule §10.A.2)
    //   - `wasm-remote-lease-late-ack` event is emitted
    //   - balances unchanged from the post-refund state

    panic!(
        "Phase 3 must implement the late-ack absorber driver against \
         the OpenFailed terminal"
    );
}

// Silence "unused import" warnings while the module is unregistered.
#[allow(dead_code)]
fn _unused_imports_anchor(
    _: PaymentGroup,
    _: ExecuteMsg,
    _: ContractError,
    _: RemoteLeaseCallback,
    _: OperationResponse,
    _: OpenLeaseResponse,
    _: RemoteLeaseId,
    _: Addr,
    _: AppResponse,
    _: ResponseWithInterChainMsgs<'_, AppResponse>,
    _: LeaseTestCase,
) -> Option<&'static str> {
    let _ = common::native_cwcoin(0);
    let _ = USER;
    let _ = testing::user("anchor");
    None
}
