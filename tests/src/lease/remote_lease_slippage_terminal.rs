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

use crate::common::testing;
use access_control::error::Error as AccessError;
use dex::{Error as DexError, MaxSlippage};
use finance::{coin::Amount, duration::Duration, fraction::Unit, price};
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
    cw_multi_test::AppResponse,
};

use crate::common::{
    self, ADMIN, LEASE_ADMIN, USER,
    leaser::Instantiator as LeaserInstantiator,
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

/// The literal-floor opening amounts of the full-liquidation driver: the base
/// is chosen close to the asset amount to trigger a full liquidation (mirrors
/// `liquidation::price`). The close swap sells the whole
/// `FULL_LIQ_LEASE_AMOUNT`.
const FULL_LIQ_LEASE_AMOUNT: Amount = 2428571428570;
const FULL_LIQ_BORROWED_AMOUNT: Amount = 1857142857142;

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

// --- A3 regression: opening (BuyAsset) timeout stays unbounded; a
//     zero-acked error unwinds (issue #658) -----------------------------

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

/// An `OperationErr` on the FIRST opening-swap leg (zero-acked,
/// `total_out == 0`) CLEAN-UNWINDS (issue #658) instead of re-emitting or
/// parking: it enters the `OpeningUnwind` drain (`OngoingTrx::Unwinding`) and
/// emits the first transfer-out to drain the inputs home. The error and
/// timeout paths stay structurally separate - the opening timeout still
/// re-emits unbounded (`buy_asset_timeout_reemits_unbounded`). The partial
/// (leg-2, `total_out > 0`) error still parks - see
/// `remote_lease_opening_terminal::opening_error_parks`.
#[test]
fn buy_asset_zero_acked_error_unwinds() {
    let (mut test_case, lease, controller) = start_open_with_delayed_swap();
    let swaps_before = recorded_swap_count(&test_case, &lease);

    // Hold the drain's transfer-out in flight so only the FIRST leg is emitted
    // on entry; under the default `Ok` mode the stand-in would ack inline and
    // chain straight to the second leg.
    stub::set_response_mode(
        &mut test_case.app,
        &controller,
        op_tag::TRANSFER_OUT,
        ResponseMode::Delayed,
    );

    let reason = RemoteErrorMessage::new("opening swap failed").expect("within length cap");
    let _unwound = stub::inject_callback(
        &mut test_case.app,
        &controller,
        &lease,
        RemoteLeaseCallback {
            nonce: 0,
            outcome: RemoteOperationOutcome::OperationErr(reason),
        },
    );

    // a zero-acked opening error does not re-emit the swap - it unwinds
    assert_eq!(
        swaps_before,
        recorded_swap_count(&test_case, &lease),
        "a zero-acked opening swap error must unwind, not re-emit"
    );
    assert_opening_unwinding(&test_case, &lease);
    // the unwind drains the inputs home: the first transfer-out is emitted
    assert_eq!(
        1,
        stub::recorded_transfer_outs(&test_case.app, &controller, &lease).len(),
        "the unwind must emit the first drain transfer-out on entry",
    );
}

// --- #660 liquidation auto-requote on timeout ----------------------------
//
// On each bounded timeout of a LIQUIDATION swap the floor re-quotes from the
// live oracle (`AcceptUpToMaxSlippage(max_slippage.liquidation)` per round,
// no episode cap, no monotonic clamp); every other flow keeps the verbatim
// pinned-floor re-emission. RED-behavioral tests fail on the floor staying
// pinned; the verbatim guards are green today and must stay green.

/// #660 (I1): an in-budget timeout of a liquidation leg re-quotes the floor
/// from the live oracle. The price DROPPED between emissions, so the fresh
/// floor is LOWER than the pinned one — no monotonic clamp; the liquidation
/// lowers its promise to clear. RED until #660 lands.
#[test]
fn liquidation_timeout_requotes_the_floor_down() {
    let mut test_case = create_test_case();
    let lease = drive_into_liquidation_swap(&mut test_case);
    let floor_before = latest_min_out(&test_case, &lease);

    let (base, quote) = (LeaseCoin::new(2), LpnCoin::new(1));
    feed_price_quietly(&mut test_case, base, quote);
    let _reemit = inject_timeout(&mut test_case, &lease);

    let floor_after = latest_min_out(&test_case, &lease);
    let expected = liquidation_floor_at(LeaseCoin::new(FULL_LIQ_LEASE_AMOUNT), base, quote);
    assert_eq!(
        expected, floor_after,
        "the re-emission must promise the fresh-quote floor, was {floor_before}",
    );
    assert!(
        floor_after < floor_before,
        "a dropped price must LOWER the requoted floor, was {floor_before}, now {floor_after}",
    );
    assert_liquidation_swap_in_flight(&test_case, &lease);
}

/// #660 (I2): the requote also tracks the oracle UP — a risen price tightens
/// the promise of the re-emitted leg. RED until #660 lands.
#[test]
fn liquidation_timeout_requotes_the_floor_up() {
    let mut test_case = create_test_case();
    let lease = drive_into_liquidation_swap(&mut test_case);
    let floor_before = latest_min_out(&test_case, &lease);

    let (base, quote) = (LeaseCoin::new(1), LpnCoin::new(2));
    feed_price_quietly(&mut test_case, base, quote);
    let _reemit = inject_timeout(&mut test_case, &lease);

    let floor_after = latest_min_out(&test_case, &lease);
    let expected = liquidation_floor_at(LeaseCoin::new(FULL_LIQ_LEASE_AMOUNT), base, quote);
    assert_eq!(
        expected, floor_after,
        "the re-emission must promise the fresh-quote floor, was {floor_before}",
    );
    assert!(
        floor_before < floor_after,
        "a risen price must RAISE the requoted floor, was {floor_before}, now {floor_after}",
    );
    assert_liquidation_swap_in_flight(&test_case, &lease);
}

/// #660 (I3): a partial (overdue) liquidation that requotes DOWN on a
/// timeout and then CLEARS at the fresh floor settles exactly as an
/// untimed-out one: the proceeds repay the overdue+due interest in full and
/// the position continues with the liquidated slice removed (mirrors
/// `liquidation::time`). RED until #660 lands (the floor stays pinned on the
/// way).
#[test]
fn partial_liquidation_requote_then_clear_settles_the_position() {
    const DOWNPAYMENT_AMOUNT: Amount = 1_000_000_000;
    // the total interest due for LeaserInstantiator::REPAYMENT_PERIOD =
    // (7% + 3%) * 65/(100-65) * downpayment * REPAYMENT_PERIOD/365
    const LIQUIDATION_AMOUNT: Amount = 45792562;
    let mut test_case = create_test_case();
    let lease = super::open_lease(
        &mut test_case,
        common::coin::<PaymentCurrency>(DOWNPAYMENT_AMOUNT),
        None,
    );
    let controller = test_case.address_book.remote_lease_controller().clone();

    let StateResponse::Opened {
        amount: lease_amount,
        ..
    } = super::state_query(&test_case, lease.clone())
    else {
        unreachable!()
    };
    let lease_amount: LeaseCoin = lease_amount.try_into().unwrap();

    test_case
        .app
        .time_shift(LeaserInstantiator::REPAYMENT_PERIOD);
    super::feed_price(&mut test_case);
    stub::set_response_mode(
        &mut test_case.app,
        &controller,
        op_tag::SWAP,
        ResponseMode::Delayed,
    );
    // the time alarm starts the overdue partial liquidation; the sell-asset
    // swap is held in flight by the Delayed mode
    let () = test_case
        .app
        .execute(
            test_case.address_book.time_alarms().clone(),
            lease.clone(),
            &ExecuteMsg::TimeAlarm {},
            &[],
        )
        .unwrap()
        .ignore_response()
        .unwrap_response();
    assert_liquidation_swap_in_flight(&test_case, &lease);
    assert_eq!(
        LIQUIDATION_AMOUNT,
        super::recorded_close_swap(&test_case, &lease)
            .coin_in()
            .amount(),
        "the overdue liquidation must sell exactly the interest-due slice",
    );

    // the price halves; the timeout requotes the slice's floor down
    let (base, quote) = (LeaseCoin::new(2), LpnCoin::new(1));
    feed_price_quietly(&mut test_case, base, quote);
    let _reemit = inject_timeout(&mut test_case, &lease);
    assert_eq!(
        liquidation_floor_at(LeaseCoin::new(LIQUIDATION_AMOUNT), base, quote),
        latest_min_out(&test_case, &lease),
        "the re-emission must promise the fresh-quote floor",
    );

    // restore the identity price so the settlement mirrors the untimed-out
    // flow, then clear the re-emitted leg — the identity payout is well
    // above the requoted floor
    super::feed_price(&mut test_case);
    let _ack = stub::deliver_pending_callback(&mut test_case.app, &controller, op_tag::SWAP);
    let (proceeds, liquidation_end): (LpnCoin, AppResponse) =
        super::settle_close_proceeds(&mut test_case, &lease);
    assert_eq!(LpnCoin::new(LIQUIDATION_AMOUNT), proceeds);
    expect_attribute(
        &liquidation_end.events,
        "wasm-ls-liquidation",
        "amount-amount",
        &LIQUIDATION_AMOUNT.to_string(),
    );

    // the slice is gone, the dues are settled, the position lives on
    match super::state_query(&test_case, lease) {
        StateResponse::Opened {
            amount,
            due_interest,
            due_margin,
            overdue_interest,
            overdue_margin,
            ..
        } => {
            assert_eq!(
                lease_amount - LeaseCoin::new(LIQUIDATION_AMOUNT),
                LeaseCoin::try_from(amount).unwrap(),
            );
            assert!(due_interest.is_zero());
            assert!(due_margin.is_zero());
            assert!(overdue_interest.is_zero());
            assert!(overdue_margin.is_zero());
        }
        other => panic!("expected the position to continue opened, got {other:?}"),
    }
}

/// #660 (I4): a FULL liquidation that requotes DOWN on a timeout and then
/// CLEARS at the fresh floor drives to the `Liquidated` end state exactly as
/// an untimed-out one (mirrors `liquidation::price::full_liquidation`). RED
/// until #660 lands (the floor stays pinned on the way).
#[test]
fn full_liquidation_requote_then_clear_liquidates() {
    let mut test_case = create_test_case();
    let lease = drive_into_liquidation_swap(&mut test_case);
    let controller = test_case.address_book.remote_lease_controller().clone();

    let (base, quote) = (LeaseCoin::new(2), LpnCoin::new(1));
    feed_price_quietly(&mut test_case, base, quote);
    let _reemit = inject_timeout(&mut test_case, &lease);
    assert_eq!(
        liquidation_floor_at(LeaseCoin::new(FULL_LIQ_LEASE_AMOUNT), base, quote),
        latest_min_out(&test_case, &lease),
        "the re-emission must promise the fresh-quote floor",
    );

    // the counterparty clears the re-emitted leg; the identity payout is
    // well above the requoted floor
    let _ack = stub::deliver_pending_callback(&mut test_case.app, &controller, op_tag::SWAP);
    let (proceeds, arrival): (LpnCoin, AppResponse) =
        super::settle_close_proceeds(&mut test_case, &lease);
    assert_eq!(LpnCoin::new(FULL_LIQ_LEASE_AMOUNT), proceeds);
    arrival.assert_event(&Event::new("wasm-ls-liquidation").add_attribute("loan-close", "true"));

    assert!(
        matches!(
            super::state_query(&test_case, lease),
            StateResponse::Liquidated()
        ),
        "the requoted-then-cleared full liquidation must end Liquidated",
    );
}

/// #660 (I5): a customer-close leg keeps the verbatim pinned floor across
/// timeouts — the `AcceptAnyNonZeroSwap` constant floor of 1 never moves,
/// oracle move or not. Green today; must stay green.
#[test]
fn customer_close_floor_stays_verbatim_across_timeouts() {
    let mut test_case = create_test_case();
    let lease = drive_into_customer_close_swap(&mut test_case);
    let floor_before = latest_min_out(&test_case, &lease);
    assert_eq!(1, floor_before, "AcceptAnyNonZeroSwap pins a floor of 1");

    feed_price_quietly(&mut test_case, LeaseCoin::new(1), LpnCoin::new(2));
    let _first = inject_timeout(&mut test_case, &lease);
    let _second = inject_timeout(&mut test_case, &lease);

    assert_eq!(
        floor_before,
        latest_min_out(&test_case, &lease),
        "a customer-close timeout must re-emit the pinned floor verbatim",
    );
    assert_customer_close_swap_in_flight(&test_case, &lease);
}

/// #660 (I6): a repay (`BuyLpn`) leg keeps the verbatim pinned floor across
/// timeouts — the constant floor of 1 never moves. This is the behavioral
/// pin of `BuyLpn::requote_on_timeout() == false`. Green today; must stay
/// green.
#[test]
fn repay_floor_stays_verbatim_across_timeouts() {
    let mut test_case = create_test_case();
    let lease = drive_into_repay_swap(&mut test_case);
    let floor_before = latest_min_out(&test_case, &lease);
    assert_eq!(1, floor_before, "AcceptAnyNonZeroSwap pins a floor of 1");

    feed_price_quietly(&mut test_case, LeaseCoin::new(1), LpnCoin::new(2));
    let _reemit = inject_timeout(&mut test_case, &lease);

    assert_eq!(
        floor_before,
        latest_min_out(&test_case, &lease),
        "a repay timeout must re-emit the pinned floor verbatim",
    );
    assert_repay_swap_in_flight(&test_case, &lease);
}

/// #660 (I7): an opening (`BuyAsset`) leg keeps the verbatim pinned floor
/// across timeouts even though its calculator reads the oracle — a moved
/// price must NOT leak into the re-emission; detection-first opening
/// semantics are preserved. Green today; must stay green.
#[test]
fn opening_floor_stays_verbatim_across_timeouts() {
    let (mut test_case, lease, _controller) = start_open_with_delayed_swap();
    let floor_before = latest_min_out(&test_case, &lease);

    // move the asset price so a requote WOULD change the floor
    feed_price_quietly(&mut test_case, LeaseCoin::new(1), LpnCoin::new(2));
    let _reemit = inject_timeout(&mut test_case, &lease);

    assert_eq!(
        floor_before,
        latest_min_out(&test_case, &lease),
        "an opening timeout must re-emit the pinned floor verbatim",
    );
    assert_opening_swap_in_flight(&test_case, &lease);
}

/// #660 (I8): requote rounds spend the same timeout budget; the round past
/// it parks with the LAST requoted floor still promised, and the parked
/// query keeps the `SlippageProtectionActivated` shape. RED until #660
/// lands.
#[test]
fn park_after_budget_holds_the_last_requoted_floor() {
    let mut test_case = create_test_case();
    let lease = drive_into_liquidation_swap(&mut test_case);

    feed_price_quietly(&mut test_case, LeaseCoin::new(2), LpnCoin::new(1));
    for round in 0..(LIQUIDATION_BUDGET - 1) {
        let _reemit = inject_timeout(&mut test_case, &lease);
        assert!(
            !is_slippage_protection_activated(&test_case, &lease),
            "the lease must still be retrying within budget, round {}",
            round + 1,
        );
    }

    // the price moves once more before the last in-budget round
    let (base, quote) = (LeaseCoin::new(4), LpnCoin::new(1));
    feed_price_quietly(&mut test_case, base, quote);
    let _reemit = inject_timeout(&mut test_case, &lease);
    assert!(!is_slippage_protection_activated(&test_case, &lease));
    let last_floor = latest_min_out(&test_case, &lease);
    assert_eq!(
        liquidation_floor_at(LeaseCoin::new(FULL_LIQ_LEASE_AMOUNT), base, quote),
        last_floor,
        "the last in-budget round must promise the fresh-quote floor",
    );

    // the round past the budget parks without re-emitting — the last
    // requoted floor stays the standing promise
    let swaps_at_budget = recorded_swap_count(&test_case, &lease);
    let _park = inject_timeout(&mut test_case, &lease);
    assert_eq!(
        swaps_at_budget,
        recorded_swap_count(&test_case, &lease),
        "the timeout past the budget must park, not re-emit",
    );
    assert_slippage_protection_activated(&test_case, &lease);
    assert_eq!(last_floor, latest_min_out(&test_case, &lease));
}

/// #660 (I9): the operator heal of a parked liquidation is UNCHANGED by the
/// auto-requote: it still re-quotes from the live oracle, resets the
/// counters, and re-enters the live swap sequence. Green today; must stay
/// green — and it pins the exact fresh-quote floor arithmetic the requote
/// tests reuse.
#[test]
fn heal_after_park_still_requotes_and_resets() {
    let mut test_case = create_test_case();
    let lease = park_liquidation(&mut test_case);

    let (base, quote) = (LeaseCoin::new(1), LpnCoin::new(2));
    feed_price_quietly(&mut test_case, base, quote);
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

    assert_eq!(
        liquidation_floor_at(LeaseCoin::new(FULL_LIQ_LEASE_AMOUNT), base, quote),
        latest_min_out(&test_case, &lease),
        "the heal must re-quote the floor from the live oracle",
    );
    assert_liquidation_swap_in_flight(&test_case, &lease);
}

/// #660 (I10): a requote round's retry event carries the previous and the
/// fresh floor as coin attributes — `min-out-prev-*` and `min-out-*`,
/// additive to the existing `timeout = retry`. RED until #660 lands.
#[test]
fn requote_round_emits_previous_and_fresh_floor() {
    let mut test_case = create_test_case();
    let lease = drive_into_liquidation_swap(&mut test_case);
    let floor_before = latest_min_out(&test_case, &lease);

    let (base, quote) = (LeaseCoin::new(2), LpnCoin::new(1));
    feed_price_quietly(&mut test_case, base, quote);
    let reemit = inject_timeout(&mut test_case, &lease);

    expect_attribute(&reemit.events, LIQUIDATION_SWAP_EVENT, "timeout", "retry");
    expect_attribute(
        &reemit.events,
        LIQUIDATION_SWAP_EVENT,
        "min-out-prev-amount",
        &floor_before.to_string(),
    );
    expect_attribute(
        &reemit.events,
        LIQUIDATION_SWAP_EVENT,
        "min-out-amount",
        &liquidation_floor_at(LeaseCoin::new(FULL_LIQ_LEASE_AMOUNT), base, quote).to_string(),
    );
}

/// #660 (I10, fallback): with the oracle's price EXPIRED (feed validity is
/// 5s x 12 samples in the test oracle), the requote's quote fails; the round
/// falls back to the pinned floor, still goes out, and marks the skipped
/// requote with `requote = skipped`. RED until #660 lands.
#[test]
fn expired_price_requote_falls_back_to_the_pinned_floor() {
    let mut test_case = create_test_case();
    let lease = drive_into_liquidation_swap(&mut test_case);
    let floor_before = latest_min_out(&test_case, &lease);

    // outlive the feed validity so the synchronous quote at the timeout
    // delivery fails
    test_case.app.time_shift(Duration::from_secs(3_600));
    let reemit = inject_timeout(&mut test_case, &lease);

    expect_attribute(&reemit.events, LIQUIDATION_SWAP_EVENT, "requote", "skipped");
    assert_eq!(
        floor_before,
        latest_min_out(&test_case, &lease),
        "an oracle failure at requote must fall back to the pinned floor",
    );
    assert_liquidation_swap_in_flight(&test_case, &lease);
}

// === drivers =============================================================

fn create_test_case() -> LeaseTestCase {
    super::create_test_case::<PaymentCurrency>()
}

/// Feed a fresh `base <-> quote` observation WITHOUT dispatching price
/// alarms — the #660 requote reads the oracle synchronously at timeout
/// delivery, so no alarm round-trip is involved and the in-flight swap state
/// stays untouched.
fn feed_price_quietly(test_case: &mut LeaseTestCase, base: LeaseCoin, quote: LpnCoin) {
    let _feed = common::oracle::feed_price(test_case, testing::user(ADMIN), base, quote);
}

/// Inject an `OperationTimeout` for the lease's in-flight leg (the stub
/// remaps the nonce onto the last emitted one).
fn inject_timeout(test_case: &mut LeaseTestCase, lease: &Addr) -> AppResponse {
    let controller = test_case.address_book.remote_lease_controller().clone();
    stub::inject_callback(
        &mut test_case.app,
        &controller,
        lease,
        RemoteLeaseCallback {
            nonce: 0,
            outcome: RemoteOperationOutcome::OperationTimeout,
        },
    )
}

/// The floor a fresh liquidation (re-)quote must pin: the liquidation
/// max-slippage bound applied to the live-oracle quote of the in-flight leg.
fn liquidation_floor_at(coin_in: LeaseCoin, base: LeaseCoin, quote: LpnCoin) -> Amount {
    MaxSlippage::unchecked(LeaserInstantiator::MAX_SLIPPAGE)
        .min_out(
            price::total(coin_in, price::total_of(base).is(quote)).expect("a representable quote"),
        )
        .to_primitive()
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

    let () = super::deliver_new_price(
        test_case,
        common::coin(FULL_LIQ_LEASE_AMOUNT - 2),
        common::coin(FULL_LIQ_BORROWED_AMOUNT),
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
    let exp_borrow = super::quote_borrow(&test_case, DOWNPAYMENT);
    let _ = common::lease::fund_remote_lease::<PaymentCurrency, LpnCurrency>(
        &mut test_case.app,
        lease.clone(),
        ica_addr,
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
fn assert_opening_unwinding(test_case: &LeaseTestCase, lease: &Addr) {
    use lease::api::query::opening::OngoingTrx;
    match super::state_query(test_case, lease.clone()) {
        StateResponse::Opening {
            in_progress: OngoingTrx::Unwinding,
            ..
        } => (),
        other => panic!("expected the opening unwind drain, got {other:?}"),
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
