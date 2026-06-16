//! Slippage-anomaly terminal E2E for the three opened remote-swap legs
//! (issue #655 Phase 1).
//!
//! The remote-lease swap legs - repay (`BuyLpn`), liquidation and
//! customer-close (`SellAsset`) - run one in-flight leg at a time over the
//! remote-lease controller. Today an `OperationErr` and an
//! `OperationTimeout` both re-emit the in-flight leg unbounded; there is no
//! terminal, so a persistently failing swap retries forever. Phase 1
//! restores the slippage valve:
//!
//! - an `OperationErr` routes immediately to a parked slippage-anomaly
//!   terminal (no retry),
//! - an `OperationTimeout` re-emits the in-flight leg up to a per-op budget
//!   (liquidation 5, customer-close 2, repay 3) and then parks,
//! - a parked lease reports `Status::SlippageProtectionActivated`, rejects
//!   customer self-rescue (`Repay`, `ClosePosition`, `ChangeClosePolicy`)
//!   and silently drops price alarms (emitting a dropped-alarm event),
//! - `Heal` is operator-only on a parked lease: a non-admin caller is
//!   rejected by the live leaser authz, the lease admin re-quotes the
//!   in-flight leg with a fresh oracle floor (meaningful for liquidation,
//!   a no-op for the `AcceptAnyNonZeroSwap` repay/customer-close legs) and
//!   resets the retry counters,
//! - the terminal absorbs late acknowledgments of the original packet.
//!
//! These drivers exercise the stable public surface - `ExecuteMsg`
//! (`Repay`, `ClosePosition`, `Heal`, `PriceAlarm`, `RemoteLeaseCallback`),
//! the `StateResponse`/`Status` query, and the controller stub's response
//! modes - so they compile against the current code and FAIL until the
//! Phase-1 behaviour lands.

use access_control::error::Error as AccessError;
use dex::Error as DexError;
use finance::coin::Amount;
use lease::{
    api::{
        ExecuteMsg,
        position::{FullClose, PositionClose},
        query::{StateResponse, opened::Status},
    },
    error::ContractError,
};
use remote_lease::callback::{RemoteErrorMessage, RemoteLeaseCallback, RemoteOperationOutcome};
use sdk::{
    cosmwasm_std::{Addr, Event},
    testing,
};

use crate::common::{
    self, LEASE_ADMIN, USER,
    remote_lease_controller_stub::{self as stub, ResponseMode, op_tag},
    test_case::TestCase,
};

use super::{DOWNPAYMENT, LeaseCoin, LeaseTestCase, LpnCoin, LpnCurrency, PaymentCurrency, repay};

const LIQUIDATION_SWAP_EVENT: &str = "wasm-ls-liquidation-swap";
const CLOSE_POSITION_EVENT: &str = "wasm-ls-close-position";

/// The per-op timeout retry budgets settled by issue #655: the number of
/// timeout re-emissions a leg tolerates before parking at the terminal.
const LIQUIDATION_BUDGET: u8 = 5;
const CUSTOMER_CLOSE_BUDGET: u8 = 2;
const REPAY_BUDGET: u8 = 3;

// --- #3 error -> immediate terminal -------------------------------------

/// An `OperationErr` on a liquidation sell-asset leg routes straight to the
/// parked terminal without a retry: the query reports
/// `SlippageProtectionActivated`.
#[test]
fn error_routes_immediately_to_terminal() {
    let mut test_case = create_test_case();
    let controller = test_case.address_book.remote_lease_controller().clone();
    let lease = drive_into_liquidation_swap(&mut test_case);

    let swaps_before = recorded_swap_count(&test_case, &lease);
    let reason = RemoteErrorMessage::new("swap reverted: under floor").expect("within length cap");
    let _absorbed = stub::inject_callback(
        &mut test_case.app,
        &controller,
        &lease,
        RemoteLeaseCallback {
            nonce: 0,
            outcome: RemoteOperationOutcome::OperationErr(reason),
        },
    );

    // an error must NOT re-emit the leg - it parks immediately
    assert_eq!(swaps_before, recorded_swap_count(&test_case, &lease));
    assert_slippage_protection_activated(&test_case, &lease);
}

// --- #4/#5/#6 timeout budget -> terminal --------------------------------

/// A liquidation leg re-emits on each `OperationTimeout` up to the budget
/// (5) and parks on the next one: the leg is still retrying right at the
/// budget and parked one beyond it.
#[test]
fn timeout_budget_then_terminal_liquidation() {
    let mut test_case = create_test_case();
    let lease = drive_into_liquidation_swap(&mut test_case);

    assert_parks_after_budget(&mut test_case, &lease, LIQUIDATION_BUDGET);
}

/// A customer-close leg parks after its budget (2).
#[test]
fn timeout_budget_then_terminal_customer_close() {
    let mut test_case = create_test_case();
    let lease = drive_into_customer_close_swap(&mut test_case);

    assert_parks_after_budget(&mut test_case, &lease, CUSTOMER_CLOSE_BUDGET);
}

/// A repay leg parks after its budget (3).
#[test]
fn timeout_budget_then_terminal_repay() {
    let mut test_case = create_test_case();
    let lease = drive_into_repay_swap(&mut test_case);

    assert_parks_after_budget(&mut test_case, &lease, REPAY_BUDGET);
}

// --- #7 query surfaces the terminal -------------------------------------

/// A parked lease answers the state query with
/// `Status::SlippageProtectionActivated`.
#[test]
fn terminal_reports_slippage_protection_activated() {
    let mut test_case = create_test_case();
    let lease = park_liquidation(&mut test_case);

    assert_slippage_protection_activated(&test_case, &lease);
}

// --- #10/#11/#12 operator-gated re-quoting heal -------------------------

/// A non-admin `Heal` on a parked lease is rejected by the live leaser
/// authz - the customer cannot self-rescue a parked slippage anomaly.
#[test]
fn heal_rejects_non_admin() {
    let mut test_case = create_test_case();
    let lease = park_liquidation(&mut test_case);

    let err = test_case
        .app
        .execute(testing::user(USER), lease.clone(), &ExecuteMsg::Heal(), &[])
        .expect_err("a non-admin heal of a parked lease must be rejected");
    assert!(
        matches!(
            err.downcast_ref::<ContractError>(),
            Some(ContractError::DexError(DexError::Unauthorized(
                AccessError::Unauthorized {}
            )))
        ),
        "expected DexError::Unauthorized, got {err:?}"
    );
    assert_slippage_protection_activated(&test_case, &lease);
}

/// The lease admin heals a parked liquidation: the leg is re-emitted with a
/// fresh oracle floor (the `percent::Calculator` re-quote pins a higher
/// floor when the oracle price has moved up) and the retry counters reset,
/// so the lease leaves the terminal back into the live swap sequence.
#[test]
fn heal_accepts_admin_requotes_and_resets_counters() {
    let mut test_case = create_test_case();
    let controller = test_case.address_book.remote_lease_controller().clone();
    let lease = park_liquidation(&mut test_case);
    let floor_before = latest_min_out(&test_case, &lease);

    // raise the asset price so a fresh quote pins a strictly higher floor
    let () = super::deliver_new_price(&mut test_case, LeaseCoin::new(1), LpnCoin::new(2))
        .ignore_response()
        .unwrap_response();

    let healed = test_case
        .app
        .execute(
            testing::user(LEASE_ADMIN),
            lease.clone(),
            &ExecuteMsg::Heal(),
            &[],
        )
        .expect("the lease admin must heal a parked lease")
        .unwrap_response();
    expect_attribute(&healed.events, LIQUIDATION_SWAP_EVENT, "heal", "re-emit");

    // the re-quote raised the floor and the lease is back to retrying
    let floor_after = latest_min_out(&test_case, &lease);
    assert!(
        floor_after > floor_before,
        "the heal re-quote must raise the liquidation floor, was {floor_before}, now {floor_after}"
    );
    assert_liquidation_swap_in_flight(&test_case, &lease);

    // the counters reset: the leg tolerates a full fresh budget again
    assert_parks_after_budget(&mut test_case, &lease, LIQUIDATION_BUDGET);
    let _ = controller;
}

/// Healing a parked customer-close leg re-emits with the same `min_out` of
/// `1`: the `AcceptAnyNonZeroSwap` floor is constant, so a re-quote is a
/// no-op for repay/customer-close.
#[test]
fn heal_requote_is_noop_for_accept_any_nonzero_floor() {
    let mut test_case = create_test_case();
    let lease = park_customer_close(&mut test_case);
    let floor_before = latest_min_out(&test_case, &lease);

    let () = super::deliver_new_price(&mut test_case, LeaseCoin::new(1), LpnCoin::new(2))
        .ignore_response()
        .unwrap_response();

    let healed = test_case
        .app
        .execute(
            testing::user(LEASE_ADMIN),
            lease.clone(),
            &ExecuteMsg::Heal(),
            &[],
        )
        .expect("the lease admin must heal a parked lease")
        .unwrap_response();
    expect_attribute(&healed.events, CLOSE_POSITION_EVENT, "heal", "re-emit");

    let floor_after = latest_min_out(&test_case, &lease);
    assert_eq!(
        1, floor_after,
        "AcceptAnyNonZeroSwap pins a constant floor of 1"
    );
    assert_eq!(
        floor_before, floor_after,
        "a re-quote must not change the constant floor"
    );
}

// --- #13/#14 full park ---------------------------------------------------

/// A parked lease rejects customer self-rescue: `Repay` surfaces the exact
/// `ContractError::UnsupportedOperation("repay")`.
#[test]
fn full_park_rejects_repay_close_policy() {
    let mut test_case = create_test_case();
    let lease = park_liquidation(&mut test_case);

    let payment = super::create_payment_coin(1_000);
    test_case.send_funds_from_admin(testing::user(USER), &[common::cwcoin(payment)]);
    let err = test_case
        .app
        .execute(
            testing::user(USER),
            lease.clone(),
            &ExecuteMsg::Repay {},
            &[common::cwcoin(payment)],
        )
        .expect_err("a parked lease must reject repay");
    assert!(
        matches!(
            err.downcast_ref::<ContractError>(),
            Some(ContractError::UnsupportedOperation(op)) if op == "repay"
        ),
        "expected UnsupportedOperation(\"repay\"), got {err:?}"
    );

    let err = test_case
        .app
        .execute(
            testing::user(USER),
            lease.clone(),
            &ExecuteMsg::ClosePosition(PositionClose::FullClose(FullClose {})),
            &[],
        )
        .expect_err("a parked lease must reject close-position");
    assert!(
        matches!(
            err.downcast_ref::<ContractError>(),
            Some(ContractError::UnsupportedOperation(op)) if op == "close position"
        ),
        "expected UnsupportedOperation(\"close position\"), got {err:?}"
    );
}

/// A parked lease silently drops a price alarm: the alarm neither errors nor
/// advances the swap, and a dropped-alarm event is emitted for monitoring.
#[test]
fn full_park_drops_price_alarm() {
    let mut test_case = create_test_case();
    let lease = park_liquidation(&mut test_case);
    let swaps_before = recorded_swap_count(&test_case, &lease);

    let oracle = test_case.address_book.oracle().clone();
    let dropped = test_case
        .app
        .execute(oracle, lease.clone(), &ExecuteMsg::PriceAlarm(), &[])
        .expect("a parked lease must absorb, not reject, a price alarm")
        .unwrap_response();

    // the parked terminal emits a dropped-alarm event rather than swallowing
    // it bare - monitoring needs to see that the parked lease ignored a
    // price move it would normally act on
    expect_attribute(
        &dropped.events,
        LIQUIDATION_SWAP_EVENT,
        "anomaly",
        "price-alarm-dropped",
    );
    assert_eq!(swaps_before, recorded_swap_count(&test_case, &lease));
    assert_slippage_protection_activated(&test_case, &lease);
}

// --- A3 regression: opening (BuyAsset) stays unbounded -------------------

/// Opening is DEFERRED in Phase 1: an opening-swap leg keeps re-emitting on
/// every `OperationTimeout`, well past any of the opened-leg budgets, and
/// never parks. Regression guard - this passes today and must keep passing.
#[test]
fn buy_asset_timeout_reemits_unbounded() {
    let (mut test_case, lease, controller) = start_open_with_delayed_swap();
    let swaps_before = recorded_swap_count(&test_case, &lease);

    // re-emit far past the largest opened-leg budget; opening never parks
    let rounds = u32::from(LIQUIDATION_BUDGET) + 3;
    for _ in 0..rounds {
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
        swaps_before + rounds,
        recorded_swap_count(&test_case, &lease),
        "every opening timeout must re-emit - opening has no terminal in Phase 1"
    );
    assert_opening_swap_in_flight(&test_case, &lease);
}

/// An `OperationErr` on an opening-swap leg parks at the slippage-anomaly
/// terminal instead of re-emitting (#638); the error and timeout paths are
/// structurally separate - the opening timeout still re-emits unbounded.
/// Regression guard.
#[test]
fn buy_asset_error_parks() {
    let (mut test_case, lease, controller) = start_open_with_delayed_swap();
    let swaps_before = recorded_swap_count(&test_case, &lease);

    let reason = RemoteErrorMessage::new("opening swap failed").expect("within length cap");
    let _parked = stub::inject_callback(
        &mut test_case.app,
        &controller,
        &lease,
        RemoteLeaseCallback {
            nonce: 0,
            outcome: RemoteOperationOutcome::OperationErr(reason),
        },
    );

    assert_eq!(
        swaps_before,
        recorded_swap_count(&test_case, &lease),
        "an opening swap error must park, not re-emit"
    );
    assert_opening_parked(&test_case, &lease);
}

// === drivers =============================================================

fn create_test_case() -> LeaseTestCase {
    super::create_test_case::<PaymentCurrency>()
}

/// Open a lease and drop the price into a full-liquidation, holding the
/// liquidation sell-asset swap in flight via the `Delayed` SWAP mode. The
/// lease sits in the `remote_swap_only` composite awaiting the swap ack.
fn drive_into_liquidation_swap(test_case: &mut LeaseTestCase) -> Addr {
    let lease = super::open_lease(test_case, DOWNPAYMENT, None);
    let controller = test_case.address_book.remote_lease_controller().clone();
    stub::set_response_mode(
        &mut test_case.app,
        &controller,
        op_tag::SWAP,
        ResponseMode::Delayed,
    );

    // the literal-floor opening amounts; the base is chosen close to the
    // asset amount to trigger a full liquidation (mirrors liquidation::price)
    let lease_amount: Amount = 2428571428570;
    let borrowed_amount: Amount = 1857142857142;
    let () = super::deliver_new_price(
        test_case,
        common::coin(lease_amount - 2),
        common::coin(borrowed_amount),
    )
    .ignore_response()
    .unwrap_response();

    assert_liquidation_swap_in_flight(test_case, &lease);
    lease
}

/// Open a lease and issue a full `ClosePosition`, holding the sell-asset
/// swap in flight via the `Delayed` SWAP mode.
fn drive_into_customer_close_swap(test_case: &mut LeaseTestCase) -> Addr {
    let lease = super::open_lease(test_case, DOWNPAYMENT, None);
    let controller = test_case.address_book.remote_lease_controller().clone();
    stub::set_response_mode(
        &mut test_case.app,
        &controller,
        op_tag::SWAP,
        ResponseMode::Delayed,
    );

    let () = test_case
        .app
        .execute(
            testing::user(USER),
            lease.clone(),
            &ExecuteMsg::ClosePosition(PositionClose::FullClose(FullClose {})),
            &[],
        )
        .expect("close-position must start the sell-asset swap")
        .ignore_response()
        .unwrap_response();

    assert_customer_close_swap_in_flight(test_case, &lease);
    lease
}

/// Open a lease and start a full repay, holding the `BuyLpn` repay swap in
/// flight via the `Delayed` SWAP mode.
fn drive_into_repay_swap(test_case: &mut LeaseTestCase) -> Addr {
    let lease = super::open_lease(test_case, DOWNPAYMENT, None);
    let controller = test_case.address_book.remote_lease_controller().clone();
    stub::set_response_mode(
        &mut test_case.app,
        &controller,
        op_tag::SWAP,
        ResponseMode::Delayed,
    );

    let payment = super::create_payment_coin(1_000);
    test_case.send_funds_from_admin(testing::user(USER), &[common::cwcoin(payment)]);
    let _repay = repay::send_repay(test_case, lease.clone(), payment);
    repay::consume_repay_swap_input(
        test_case,
        &TestCase::ica_addr(&lease, TestCase::LEASE_ICA_ID),
        payment,
    );

    assert_repay_swap_in_flight(test_case, &lease);
    lease
}

/// Drive into the liquidation swap and park it by exhausting the timeout
/// budget (budget + 1 timeouts).
fn park_liquidation(test_case: &mut LeaseTestCase) -> Addr {
    let lease = drive_into_liquidation_swap(test_case);
    exhaust_budget(test_case, &lease, LIQUIDATION_BUDGET);
    lease
}

/// Drive into the customer-close swap and park it.
fn park_customer_close(test_case: &mut LeaseTestCase) -> Addr {
    let lease = drive_into_customer_close_swap(test_case);
    exhaust_budget(test_case, &lease, CUSTOMER_CLOSE_BUDGET);
    lease
}

/// Open a lease, hold the opening swaps in flight via the `Delayed` SWAP
/// mode, and confirm an opening swap leg is outstanding.
fn start_open_with_delayed_swap() -> (LeaseTestCase, Addr, Addr) {
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

    assert_opening_swap_in_flight(&test_case, &lease);
    (test_case, lease, controller)
}

/// Inject `budget` timeouts (each must re-emit, still retrying) and assert
/// the `budget + 1`th parks the lease at the terminal.
#[track_caller]
fn assert_parks_after_budget(test_case: &mut LeaseTestCase, lease: &Addr, budget: u8) {
    let controller = test_case.address_book.remote_lease_controller().clone();
    let swaps_before = recorded_swap_count(test_case, lease);

    for round in 0..budget {
        let _reemit = stub::inject_callback(
            &mut test_case.app,
            &controller,
            lease,
            RemoteLeaseCallback {
                nonce: 0,
                outcome: RemoteOperationOutcome::OperationTimeout,
            },
        );
        // each timeout within budget re-emits exactly one swap and the lease
        // keeps retrying (NOT parked yet)
        assert_eq!(
            swaps_before + u32::from(round) + 1,
            recorded_swap_count(test_case, lease),
            "timeout {} within budget {budget} must re-emit the in-flight leg",
            round + 1,
        );
        assert!(
            !is_slippage_protection_activated(test_case, lease),
            "the lease must still be retrying within budget {budget}, round {}",
            round + 1,
        );
    }

    // the budget+1-th timeout parks - no further re-emission
    let swaps_at_budget = recorded_swap_count(test_case, lease);
    let _park = stub::inject_callback(
        &mut test_case.app,
        &controller,
        lease,
        RemoteLeaseCallback {
            nonce: 0,
            outcome: RemoteOperationOutcome::OperationTimeout,
        },
    );
    assert_eq!(
        swaps_at_budget,
        recorded_swap_count(test_case, lease),
        "the timeout past budget {budget} must park, not re-emit",
    );
    assert_slippage_protection_activated(test_case, lease);
}

/// Drive exactly `budget + 1` timeouts to land the lease at the terminal,
/// without the per-round budget assertions.
fn exhaust_budget(test_case: &mut LeaseTestCase, lease: &Addr, budget: u8) {
    let controller = test_case.address_book.remote_lease_controller().clone();
    for _ in 0..=budget {
        let _cb = stub::inject_callback(
            &mut test_case.app,
            &controller,
            lease,
            RemoteLeaseCallback {
                nonce: 0,
                outcome: RemoteOperationOutcome::OperationTimeout,
            },
        );
    }
    assert_slippage_protection_activated(test_case, lease);
}

// === query / recorder helpers ===========================================

fn recorded_swap_count(test_case: &LeaseTestCase, lease: &Addr) -> u32 {
    let controller = test_case.address_book.remote_lease_controller();
    stub::recorded_swaps(&test_case.app, controller, lease)
        .len()
        .try_into()
        .expect("swap count fits u32")
}

/// The `min_out` floor of the latest recorded swap leg for the lease.
fn latest_min_out(test_case: &LeaseTestCase, lease: &Addr) -> Amount {
    super::recorded_close_swap(test_case, lease)
        .min_out()
        .amount()
}

#[track_caller]
fn assert_slippage_protection_activated(test_case: &LeaseTestCase, lease: &Addr) {
    match super::state_query(test_case, lease.clone()) {
        StateResponse::Opened {
            status: Status::SlippageProtectionActivated,
            ..
        } => (),
        other => panic!("expected a parked slippage-anomaly terminal, got {other:?}"),
    }
}

fn is_slippage_protection_activated(test_case: &LeaseTestCase, lease: &Addr) -> bool {
    matches!(
        super::state_query(test_case, lease.clone()),
        StateResponse::Opened {
            status: Status::SlippageProtectionActivated,
            ..
        }
    )
}

#[track_caller]
fn assert_liquidation_swap_in_flight(test_case: &LeaseTestCase, lease: &Addr) {
    use lease::api::query::opened::{OngoingTrx, PositionCloseTrx};
    match super::state_query(test_case, lease.clone()) {
        StateResponse::Opened {
            status:
                Status::InProgress(OngoingTrx::Liquidation {
                    in_progress: PositionCloseTrx::Swap,
                    ..
                }),
            ..
        } => (),
        other => panic!("expected the liquidation swap in flight, got {other:?}"),
    }
}

#[track_caller]
fn assert_customer_close_swap_in_flight(test_case: &LeaseTestCase, lease: &Addr) {
    use lease::api::query::opened::{OngoingTrx, PositionCloseTrx};
    match super::state_query(test_case, lease.clone()) {
        StateResponse::Opened {
            status:
                Status::InProgress(OngoingTrx::Close {
                    in_progress: PositionCloseTrx::Swap,
                    ..
                }),
            ..
        } => (),
        other => panic!("expected the customer-close swap in flight, got {other:?}"),
    }
}

#[track_caller]
fn assert_repay_swap_in_flight(test_case: &LeaseTestCase, lease: &Addr) {
    use lease::api::query::opened::{OngoingTrx, RepayTrx};
    match super::state_query(test_case, lease.clone()) {
        StateResponse::Opened {
            status:
                Status::InProgress(OngoingTrx::Repayment {
                    in_progress: RepayTrx::Swap,
                    ..
                }),
            ..
        } => (),
        other => panic!("expected the repay swap in flight, got {other:?}"),
    }
}

#[track_caller]
fn assert_opening_swap_in_flight(test_case: &LeaseTestCase, lease: &Addr) {
    use lease::api::query::opening::OngoingTrx;
    match super::state_query(test_case, lease.clone()) {
        StateResponse::Opening {
            in_progress: OngoingTrx::BuyAsset { .. },
            ..
        } => (),
        other => panic!("expected the opening swap in flight, got {other:?}"),
    }
}

#[track_caller]
fn assert_opening_parked(test_case: &LeaseTestCase, lease: &Addr) {
    use lease::api::query::opening::OngoingTrx;
    match super::state_query(test_case, lease.clone()) {
        StateResponse::Opening {
            in_progress: OngoingTrx::SlippageProtectionActivated,
            ..
        } => (),
        other => panic!("expected the parked opening leg, got {other:?}"),
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
