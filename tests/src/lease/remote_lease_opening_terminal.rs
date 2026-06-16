//! Slippage-anomaly terminal E2E for the OPENING remote-swap leg
//! (`BuyAsset`, issue #638).
//!
//! The opening swap runs two sequential single-coin legs over the
//! remote-lease controller: leg 1 the downpayment, leg 2 the loan
//! principal, decrementing `acks_left` per acknowledgment. Today
//! (pre-#638) an `OperationErr` on an opening leg re-emits the in-flight
//! leg unbounded - there is no terminal, so an under-floor rejection
//! retries forever. #638 reuses the opened-leg `SlippageAnomaly` terminal
//! for the opening leg, parking it on a hard error:
//!
//! - an `OperationErr` parks the in-flight opening leg immediately at the
//!   `SlippageProtectionActivated` terminal (no retry), preserving the
//!   accumulated `total_out` of the already-acked legs (complete-forward),
//! - an `OperationTimeout` is UNCHANGED - it re-emits the in-flight leg
//!   verbatim, unbounded, never parking (D2: error and timeout are
//!   structurally separate),
//! - the parked opening leg is queryable as
//!   `StateResponse::Opening { in_progress:
//!   opening::OngoingTrx::SlippageProtectionActivated, .. }` and emits the
//!   `slippage-anomaly-parked` event under the `wasm-ls-open-swap` label,
//! - `Heal` is `lease_admin`-gated on the parked opening terminal: a
//!   non-`lease_admin` caller is rejected `Unauthorized`, the lease admin
//!   re-quotes the in-flight leg against the `opening` slippage bound and
//!   re-drives it, preserving the already-acked proceeds, so the opening
//!   completes into the live `Active` lease,
//! - the terminal absorbs a late ack/err/timeout of the pre-park emission.
//!
//! These drivers exercise the stable public surface - `ExecuteMsg`
//! (`Heal`, `RemoteLeaseCallback` via the controller stand-in), the
//! `StateResponse` query, and the controller stub's response modes.
//!
//! The CHARACTERIZATION / failing-first test (#1,
//! `opening_error_reemits_unbounded_today`) lives in the sibling
//! `remote_lease_slippage_terminal` module instead - it must compile and
//! PASS against the current code, while every test here references the
//! post-#638 query variant `opening::OngoingTrx::SlippageProtectionActivated`
//! and so will NOT compile until production lands.
//!
//! Test inventory (all TARGET API - do not compile until #638 lands):
//!
//! - `opening_error_parks` (#2), `opening_error_parks_emits_anomaly_event`
//!   (#2), `opening_timeout_reemits_not_parks` (#3),
//!   `parked_opening_absorbs_late_acks` (#4),
//!   `opening_error_reason_dropped_on_park` (#5),
//!   `heal_completes_forward_to_active` (#6),
//!   `heal_rejects_non_lease_admin` (#7),
//!   `leg_one_error_parks_forward_only` (#8),
//!   `nonce_interleave_stale_error_absorbed_current_error_parks` (#9).

use access_control::error::Error as AccessError;
use dex::Error as DexError;
use finance::{coin::Amount, zero::Zero};
use lease::{
    api::{
        ExecuteMsg,
        query::{StateResponse, opening::OngoingTrx as OpeningOngoingTrx},
    },
    error::ContractError,
};
use remote_lease::callback::{RemoteErrorMessage, RemoteLeaseCallback, RemoteOperationOutcome};
use remote_lease::response::{OperationResponse, SwapResponse};
use sdk::{
    cosmwasm_std::{Addr, Event},
    cw_multi_test::AppResponse,
    testing,
};

use crate::common::{
    self, ADMIN, LEASE_ADMIN,
    remote_lease_controller_stub::{self as stub, ResponseMode, op_tag},
    test_case::TestCase,
};

use super::{DOWNPAYMENT, LeaseCoin, LeaseTestCase, LpnCurrency, PaymentCurrency};

const OPENING_SWAP_EVENT: &str = "wasm-ls-open-swap";

/// Far past any plausible retry budget - an opening leg with a terminal
/// would have parked long before this many re-emissions.
const UNBOUNDED_ROUNDS: u32 = 16;

// === #2 error -> Park terminal ==========================================

/// TARGET (#2): a deterministic `OperationErr` on the in-flight opening leg
/// parks the lease at the `SlippageProtectionActivated` terminal in bounded
/// steps - the error emits no further swap. (Behaviour assertion.)
#[test]
fn opening_error_parks() {
    let (mut test_case, lease, controller) = start_opening_leg_two_in_flight();
    let swaps_before = recorded_swap_count(&test_case, &lease);

    inject_opening_error(&mut test_case, &controller, &lease, "under floor");

    assert_eq!(
        swaps_before,
        recorded_swap_count(&test_case, &lease),
        "an opening swap error must park, not re-emit",
    );
    assert_opening_slippage_protection_activated(&test_case, &lease);
}

/// TARGET (#2): the opening leg parks under the opening swap event label
/// (`wasm-ls-open-swap`) with the on-entry `slippage-anomaly-parked` reason.
#[test]
fn opening_error_parks_emits_anomaly_event() {
    let (mut test_case, lease, controller) = start_opening_leg_two_in_flight();

    let parked = inject_opening_error(&mut test_case, &controller, &lease, "under floor");

    expect_attribute(
        &parked.events,
        OPENING_SWAP_EVENT,
        "anomaly",
        "slippage-anomaly-parked",
    );
}

// === #3 error != timeout (D2) ===========================================

/// TARGET (#3, D2): an `OperationTimeout` on the in-flight opening leg keeps
/// re-emitting verbatim, unbounded past any retry budget, and never parks -
/// contrasting #2 (error parks). Error and timeout stay structurally
/// separate.
#[test]
fn opening_timeout_reemits_not_parks() {
    let (mut test_case, lease, controller) = start_opening_leg_two_in_flight();
    let swaps_before = recorded_swap_count(&test_case, &lease);

    for _ in 0..UNBOUNDED_ROUNDS {
        let _reemit = stub::inject_callback(
            &mut test_case.app,
            &controller,
            &lease,
            RemoteLeaseCallback {
                nonce: 0,
                outcome: RemoteOperationOutcome::OperationTimeout,
            },
        );
    }

    assert_eq!(
        swaps_before + UNBOUNDED_ROUNDS,
        recorded_swap_count(&test_case, &lease),
        "every opening timeout must re-emit - opening never parks on timeout (D2)",
    );
    assert_opening_swap_in_flight(&test_case, &lease);
}

// === #4 terminal absorbs late acks ======================================

/// TARGET (#4): after the opening leg parks, a late ack of the pre-park
/// emission (carrying the parked leg's nonce) is absorbed with a
/// `parked-response` reason and does NOT re-credit or leave the terminal.
#[test]
fn parked_opening_absorbs_late_acks() {
    let (mut test_case, lease, controller) = start_opening_leg_two_in_flight();
    inject_opening_error(&mut test_case, &controller, &lease, "under floor");
    let swaps_after_park = recorded_swap_count(&test_case, &lease);

    // a late OK ack of the original parked packet (the stand-in stamps the
    // parked leg's in-flight nonce) is absorbed, not re-credited
    let late_ok = stub::inject_callback(
        &mut test_case.app,
        &controller,
        &lease,
        RemoteLeaseCallback {
            nonce: 0,
            outcome: RemoteOperationOutcome::OperationOk(
                OperationResponse::Swap(SwapResponse {
                    amount_out: super::LeaseCoin::new(1_000).into(),
                })
                .into(),
            ),
        },
    );
    expect_attribute(
        &late_ok.events,
        OPENING_SWAP_EVENT,
        "absorbed",
        "parked-response",
    );

    // a late error of the original packet is absorbed too
    let reason = RemoteErrorMessage::new("late error").expect("within length cap");
    let late_err = stub::inject_callback(
        &mut test_case.app,
        &controller,
        &lease,
        RemoteLeaseCallback {
            nonce: 0,
            outcome: RemoteOperationOutcome::OperationErr(reason),
        },
    );
    expect_attribute(
        &late_err.events,
        OPENING_SWAP_EVENT,
        "absorbed",
        "parked-error",
    );

    // neither absorbed callback re-emitted or left the terminal
    assert_eq!(
        swaps_after_park,
        recorded_swap_count(&test_case, &lease),
        "an absorbed late callback must not re-emit",
    );
    assert_opening_slippage_protection_activated(&test_case, &lease);
}

// === #5 bounded / dropped error reason ==================================

/// TARGET (#5): the counterparty error message is bounded to
/// `OPERATION_ERR_MAX_BYTES` at the wire boundary (the type rejects an
/// over-cap construction) and is DROPPED on the park path - the parked
/// event surfaces no counterparty-controlled `reason` attribute, so a
/// hostile counterparty cannot inflate events/storage through a parked
/// opening leg.
#[test]
fn opening_error_reason_dropped_on_park() {
    use remote_lease::callback::OPERATION_ERR_MAX_BYTES;

    // the wire type refuses to carry an over-cap reason: it never reaches
    // the lease in the first place
    let over_cap = "x".repeat(OPERATION_ERR_MAX_BYTES + 1);
    assert!(
        RemoteErrorMessage::new(over_cap).is_err(),
        "an over-cap counterparty reason must be rejected at the wire boundary",
    );

    // a within-cap but distinctive reason is dropped on park: the parked
    // event carries no `reason` attribute
    let (mut test_case, lease, controller) = start_opening_leg_two_in_flight();
    let secret = "counterparty-controlled-leak";
    let parked = inject_opening_error(&mut test_case, &controller, &lease, secret);

    let parked_event = parked
        .events
        .iter()
        .find(|event| event.ty == OPENING_SWAP_EVENT)
        .expect("the park must emit under the opening swap label");
    assert!(
        parked_event
            .attributes
            .iter()
            .all(|attr| attr.value != secret),
        "the counterparty error reason must be dropped on park, got {parked_event:?}",
    );
}

// === #6 heal complete-forward (LEASE_ADMIN) =============================

/// TARGET (#6): leg 1 acks (total_out > 0), leg 2 errors -> parks. The lease
/// admin heals: leg 2 is re-emitted with leg 1's proceeds preserved
/// (complete-forward) and acks, driving the opening to the live `Opened`
/// lease. The heal re-quotes a fresh leg (a strictly greater nonce, a floor
/// freshly pinned against the `opening` bound).
#[test]
fn heal_completes_forward_to_active() {
    let (mut test_case, lease, controller) = start_opening_leg_two_in_flight();
    let total_before = recorded_total_out(&test_case, &lease);
    assert!(
        total_before > LeaseCoin::ZERO,
        "leg 1 must have acked with non-zero proceeds before leg 2 parks",
    );

    inject_opening_error(&mut test_case, &controller, &lease, "under floor");
    assert_opening_slippage_protection_activated(&test_case, &lease);

    let nonce_before = latest_swap_nonce(&test_case, &lease);

    // re-emitted legs ack inline under the default `Ok` mode
    stub::set_response_mode(
        &mut test_case.app,
        &controller,
        op_tag::SWAP,
        ResponseMode::Ok,
    );
    let healed = test_case
        .app
        .execute(
            testing::user(LEASE_ADMIN),
            lease.clone(),
            &ExecuteMsg::Heal(),
            &[],
        )
        .expect("the lease admin must heal a parked opening lease")
        .unwrap_response();
    expect_attribute(&healed.events, OPENING_SWAP_EVENT, "heal", "re-emit");

    // the heal re-quoted: a fresh emission with a strictly greater nonce
    let nonce_after = latest_swap_nonce(&test_case, &lease);
    assert!(
        nonce_before < nonce_after,
        "the heal must re-emit with a strictly greater nonce, was {nonce_before}, now {nonce_after}",
    );

    // complete-forward: leg 1's proceeds were preserved, so the opened
    // amount is at least what leg 1 had already accumulated and the lease is
    // live
    let opened = opened_amount(&test_case, &lease);
    assert!(
        opened >= total_before,
        "the opened amount must preserve leg 1's proceeds, leg1 {total_before}, opened {opened}",
    );
}

// === #7 negative authz ==================================================

/// TARGET (#7): a `Heal` of a parked opening lease from a non-`lease_admin`
/// caller is rejected `Unauthorized` and leaves the lease parked. Mirrors
/// the opened-leg `heal_rejects_unauthorised_operator`; `ADMIN` is the
/// protocol admin, NOT the lease admin, so it must be rejected.
#[test]
fn heal_rejects_non_lease_admin() {
    let (mut test_case, lease, controller) = start_opening_leg_two_in_flight();
    inject_opening_error(&mut test_case, &controller, &lease, "under floor");

    let err = test_case
        .app
        .execute(
            testing::user(ADMIN),
            lease.clone(),
            &ExecuteMsg::Heal(),
            &[],
        )
        .expect_err("a non-lease-admin heal of a parked opening lease must be rejected");
    assert!(
        matches!(
            err.downcast_ref::<ContractError>(),
            Some(ContractError::DexError(DexError::Unauthorized(
                AccessError::Unauthorized {}
            )))
        ),
        "expected DexError::Unauthorized, got {err:?}",
    );
    assert_opening_slippage_protection_activated(&test_case, &lease);
}

// === #8 leg-1 forward-only park =========================================

/// TARGET (#8, Ivan's uniform-park decision): an `OperationErr` on LEG 1
/// (the downpayment leg, total_out == 0) parks forward-only - it must reach
/// the queryable terminal, NOT a clean refund and NOT `OpenFailed`. This
/// locks in the uniform-park decision so a future "add a refund for leg 1"
/// change is caught as a regression.
#[test]
fn leg_one_error_parks_forward_only() {
    let (mut test_case, lease, controller) = start_opening_leg_one_in_flight();
    let total_before = recorded_total_out(&test_case, &lease);
    assert_eq!(
        LeaseCoin::ZERO,
        total_before,
        "leg 1 parks with no acked proceeds yet (total_out == 0)",
    );

    inject_opening_error(
        &mut test_case,
        &controller,
        &lease,
        "downpayment leg under floor",
    );

    // forward-only park: a queryable terminal, NOT a refund / OpenFailed
    match super::state_query(&test_case, lease.clone()) {
        StateResponse::Opening {
            in_progress: OpeningOngoingTrx::SlippageProtectionActivated,
            ..
        } => (),
        StateResponse::OpenFailed { .. } => {
            panic!("leg 1 must PARK forward-only, not refund into OpenFailed")
        }
        other => panic!("expected the parked opening terminal, got {other:?}"),
    }
}

// === #9 nonce interleave ================================================

/// TARGET (#9): an opening timeout re-emits with a bumped nonce (N -> N+1);
/// a stale-nonce (N) error then arrives and is absorbed as `nonce-mismatch`
/// (NOT parked - it resolves a superseded packet); a current-nonce (N+1)
/// error then PARKS. Proves error-park is gated on the live in-flight nonce.
#[test]
fn nonce_interleave_stale_error_absorbed_current_error_parks() {
    let (mut test_case, lease, controller) = start_opening_leg_two_in_flight();
    let stale_nonce = latest_swap_nonce(&test_case, &lease);

    // a timeout re-emits the in-flight leg with a strictly greater nonce
    let _reemit = stub::inject_callback(
        &mut test_case.app,
        &controller,
        &lease,
        RemoteLeaseCallback {
            nonce: 0,
            outcome: RemoteOperationOutcome::OperationTimeout,
        },
    );
    let current_nonce = latest_swap_nonce(&test_case, &lease);
    assert!(
        stale_nonce < current_nonce,
        "the timeout re-emission must bump the nonce, was {stale_nonce}, now {current_nonce}",
    );
    let swaps_after_reemit = recorded_swap_count(&test_case, &lease);

    // a stale-nonce error resolves the superseded packet -> absorbed, NOT
    // parked
    let stale_reason = RemoteErrorMessage::new("superseded error").expect("within length cap");
    let stale_err = stub::inject_callback_with_nonce(
        &mut test_case.app,
        &controller,
        &lease,
        stale_nonce,
        RemoteOperationOutcome::OperationErr(stale_reason),
    );
    expect_attribute(
        &stale_err.events,
        OPENING_SWAP_EVENT,
        "absorbed",
        "nonce-mismatch",
    );
    assert_eq!(
        swaps_after_reemit,
        recorded_swap_count(&test_case, &lease),
        "a stale-nonce error must be absorbed, not re-emit or park",
    );
    assert_opening_swap_in_flight(&test_case, &lease);

    // a current-nonce error parks
    let current_reason = RemoteErrorMessage::new("under floor").expect("within length cap");
    let _park = stub::inject_callback_with_nonce(
        &mut test_case.app,
        &controller,
        &lease,
        current_nonce,
        RemoteOperationOutcome::OperationErr(current_reason),
    );
    assert_opening_slippage_protection_activated(&test_case, &lease);
}

// === drivers ============================================================

fn create_test_case() -> LeaseTestCase {
    super::create_test_case::<PaymentCurrency>()
}

/// Open a lease, hold the opening swaps in flight via `Delayed`, and ack
/// leg 1 so leg 2 (the loan-principal leg, total_out > 0) is the in-flight
/// leg. Returns with leg 2 outstanding.
fn start_opening_leg_two_in_flight() -> (LeaseTestCase, Addr, Addr) {
    let (mut test_case, lease, controller) = start_opening_leg_one_in_flight();

    // ack leg 1: advances to leg 2, which is also held by `Delayed`
    let _leg_one_ack =
        stub::deliver_pending_callback(&mut test_case.app, &controller, op_tag::SWAP);
    assert_eq!(
        1,
        opening_acks_left(&test_case, &lease),
        "leg 1 must have acked, leaving leg 2 in flight",
    );

    (test_case, lease, controller)
}

/// Open a lease and hold the opening swaps in flight via the `Delayed` SWAP
/// mode, with leg 1 (the downpayment leg, total_out == 0) outstanding.
fn start_opening_leg_one_in_flight() -> (LeaseTestCase, Addr, Addr) {
    let mut test_case = create_test_case();
    let lease = super::try_init_lease(&mut test_case, DOWNPAYMENT, None);
    let controller = test_case.address_book.remote_lease_controller().clone();
    stub::set_response_mode(
        &mut test_case.app,
        &controller,
        op_tag::SWAP,
        ResponseMode::Delayed,
    );

    let ica_addr = TestCase::ica_addr(&lease, TestCase::LEASE_ICA_ID);
    let ica_port = format!("icacontroller-{ica_addr}");
    let ica_channel = format!("channel-{ica_addr}");
    let exp_borrow = super::quote_borrow(&test_case, DOWNPAYMENT);
    let _ = common::lease::confirm_ica_and_transfer_funds::<PaymentCurrency, LpnCurrency>(
        &mut test_case.app,
        lease.clone(),
        TestCase::DEX_CONNECTION_ID,
        (&ica_channel, &ica_port, ica_addr),
        (DOWNPAYMENT, exp_borrow),
    )
    .unwrap_response();

    assert_eq!(
        2,
        opening_acks_left(&test_case, &lease),
        "both opening legs must be outstanding with leg 1 in flight",
    );
    (test_case, lease, controller)
}

/// Inject a deterministic `OperationErr` carrying the lease's current
/// in-flight swap nonce (the stand-in stamps the last recorded nonce).
fn inject_opening_error(
    test_case: &mut LeaseTestCase,
    controller: &Addr,
    lease: &Addr,
    reason: &str,
) -> AppResponse {
    let reason = RemoteErrorMessage::new(reason.to_owned()).expect("within length cap");
    stub::inject_callback(
        &mut test_case.app,
        controller,
        lease,
        RemoteLeaseCallback {
            nonce: 0,
            outcome: RemoteOperationOutcome::OperationErr(reason),
        },
    )
}

// === query / recorder helpers ===========================================

fn recorded_swap_count(test_case: &LeaseTestCase, lease: &Addr) -> u32 {
    let controller = test_case.address_book.remote_lease_controller();
    stub::recorded_swaps(&test_case.app, controller, lease)
        .len()
        .try_into()
        .expect("swap count fits u32")
}

/// The sum of the `min_out` floors of every recorded swap leg - the
/// accumulated proceeds an opening leg credits at the literal floor. Leg 1
/// acked credits its floor; leg 2 not yet acked contributes nothing to the
/// node's `total_out`, but the recorded floors are the test-side mirror of
/// the proceeds threaded through the park.
fn recorded_total_out(test_case: &LeaseTestCase, lease: &Addr) -> LeaseCoin {
    let controller = test_case.address_book.remote_lease_controller();
    let acks_left = opening_acks_left(test_case, lease);
    let swaps = stub::recorded_swaps(&test_case.app, controller, lease);
    // acked legs are every recorded leg except the in-flight (last) one
    let acked = swaps.len().saturating_sub(usize::from(acks_left));
    let summed: Amount = swaps
        .iter()
        .take(acked)
        .map(|params| params.min_out().amount())
        .sum();
    LeaseCoin::new(summed)
}

fn latest_swap_nonce(test_case: &LeaseTestCase, lease: &Addr) -> u64 {
    let controller = test_case.address_book.remote_lease_controller();
    *stub::recorded_swap_nonces(&test_case.app, controller, lease)
        .last()
        .expect("at least one swap recorded")
}

fn opening_acks_left(test_case: &LeaseTestCase, lease: &Addr) -> u8 {
    match super::state_query(test_case, lease.clone()) {
        StateResponse::Opening {
            in_progress: OpeningOngoingTrx::BuyAsset { acks_left },
            ..
        } => acks_left,
        other => panic!("expected the in-flight opening swap leg, got {other:?}"),
    }
}

fn opened_amount(test_case: &LeaseTestCase, lease: &Addr) -> LeaseCoin {
    match super::state_query(test_case, lease.clone()) {
        StateResponse::Opened { amount, .. } => amount.try_into().unwrap(),
        other => panic!("expected an opened lease, got {other:?}"),
    }
}

#[track_caller]
fn assert_opening_slippage_protection_activated(test_case: &LeaseTestCase, lease: &Addr) {
    match super::state_query(test_case, lease.clone()) {
        StateResponse::Opening {
            in_progress: OpeningOngoingTrx::SlippageProtectionActivated,
            ..
        } => (),
        other => panic!("expected the parked opening slippage-anomaly terminal, got {other:?}"),
    }
}

#[track_caller]
fn assert_opening_swap_in_flight(test_case: &LeaseTestCase, lease: &Addr) {
    match super::state_query(test_case, lease.clone()) {
        StateResponse::Opening {
            in_progress: OpeningOngoingTrx::BuyAsset { .. },
            ..
        } => (),
        other => panic!("expected the opening swap in flight, got {other:?}"),
    }
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
