//! Swap E2E (issue #142 Phase 4).
//!
//! ⚠️ UNREGISTERED ⚠️ — this module targets the **post-refactor** lease
//! state machine and will not compile on `main`. Registration in
//! `tests/src/lease/mod.rs` is intentionally **omitted**; Phase 4
//! un-comments it once the post-refactor `SwapExactIn` path lands. See
//! `.claude/handoffs/2026-05-25-issue142-plan.md`, §5 Phase 4 and
//! amendment banner 2026-05-27 (swap stays single-coin per call).
//!
//! Atomicity model (amendment banner 2026-05-27):
//!
//! `Operation::Swap` is **single-coin per call**. If the lease's
//! `SwapExactIn` path needs to swap multiple currencies, it emits N
//! sequential single-coin `Lease::swap` calls and decrements
//! `acks_left = N` per ack (mirroring TransferOut). There is NO batched
//! `Vec<SwapLeg>` packet. Tests in this file MUST NOT assume a multi-leg
//! shape.
//!
//! Acceptance criteria (plan §5 Phase 2):
//!
//! - `swap_single_coin_happy_path` — the post-refactor `SwapExactIn`
//!   state-enter emits a single `ExecuteMsg::Swap { params, timeout }`
//!   carrying the lease's coin-in / min-out; the stand-in's
//!   `OperationResponse::Swap { amount_out = min_out }` ack drives the
//!   transition to the next state.
//! - `swap_multi_currency_sequential_acks` — when the lease holds two
//!   currencies that both need swapping, `SwapExactIn` emits two
//!   sequential single-coin `Swap` calls; the lease tracks
//!   `acks_left = 2` and exposes the intermediate state via
//!   `OngoingTrx`; only after the second ack does it transition.
//! - `swap_delayed_ack_visible_in_query` — using
//!   `ResponseMode::Delayed`, the lease's `OngoingTrx` is observable at
//!   intermediate blocks (the test advances the block before
//!   `DeliverPending`).

use lease::api::query::{StateResponse, opened::OngoingTrx as OpenedOngoingTrx};
use remote_lease::{callback::RemoteLeaseCallback, response::OperationResponse};
use sdk::cosmwasm_std::Addr;

use crate::common::remote_lease_controller_stub::{self as stub, ResponseMode, op_tag};

use super::{LeaseCurrency, LeaseTestCase};

#[test]
fn swap_single_coin_happy_path() {
    let _test_case = super::create_test_case::<LeaseCurrency>();
    panic!("Phase 4 must implement single-coin Swap driver");
}

#[test]
fn swap_multi_currency_sequential_acks() {
    let _test_case = super::create_test_case::<LeaseCurrency>();
    // Drive the lease into a SwapExactIn state where it holds two
    // currencies (the buy-asset path or the close path with mixed
    // balances). Assert: two sequential `ExecuteMsg::Swap` calls, an
    // `acks_left = 2 → 1 → 0` countdown visible via OngoingTrx.
    panic!("Phase 4 must implement multi-currency sequential-call assertion");
}

#[test]
fn swap_delayed_ack_visible_in_query() {
    let mut test_case = super::create_test_case::<LeaseCurrency>();
    let controller = test_case.address_book.remote_lease_controller().clone();
    stub::set_response_mode(
        &mut test_case.app,
        &controller,
        op_tag::SWAP,
        ResponseMode::Delayed,
    );

    // Advance two blocks between the swap emission and the delayed
    // delivery; assert the lease's OngoingTrx reports the in-flight
    // swap throughout.

    panic!("Phase 4 must implement delayed-ack visibility driver");
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
