//! Clean-unwind E2E for a zero-acked opening swap error (issue #658).
//!
//! When the opening `BuyAsset` swap hits a hard remote `OperationErr` with
//! NOTHING acknowledged yet (`total_out == 0`, the FIRST swap leg in flight),
//! the lease no longer parks at the slippage terminal (the pre-#658
//! behaviour, still in force for a partial leg-2 error). Instead it
//! CLEAN-UNWINDS:
//!
//! 1. it enters a new top-level `OpeningUnwind` drain - a `dex::StateDrain`
//!    over `OpeningUnwindTask` - emitting the first transfer-out leg under the
//!    `wasm-ls-open-unwind` label,
//! 2. the drain transfers the downpayment and the loan principal back home
//!    from the Solana-side `LeaseAuthority`, one in-flight transfer at a time,
//!    over the SAME `RemoteTransferOut` / `FundsArrival` machinery the
//!    paid-close drain uses (see `remote_lease_transfer_out`),
//! 3. on full arrival - measured by a per-currency baseline+aggregate balance
//!    check - `finish()` fires a reserve-covered full close: it covers the
//!    LPP loan's accrued interest from the reserve, repays the loan IN FULL
//!    (so the loan CLOSES), refunds the WHOLE downpayment to the customer,
//!    finalises the lease, and transitions to `StateResponse::OpenFailed`,
//!    emitting `wasm-ls-remote-lease-open-failed`,
//! 4. throughout the drain `state_query` returns
//!    `StateResponse::Opening { in_progress: OngoingTrx::Unwinding, .. }`.
//!
//! A drain transfer-out ERROR is absorbed (`absorbed = remote-error` under
//! `wasm-ls-open-unwind`) and re-emitted only on `Heal`; a transfer-out
//! TIMEOUT re-emits verbatim - the same paid-close-drain recovery rules.
//!
//! These drivers exercise the stable public surface - `ExecuteMsg`
//! (`Heal`, `RemoteLeaseCallback`, `TimeAlarm`), the `StateResponse` query,
//! and the controller stub's response modes.
//!
//! Test inventory:
//!
//! - AC4 `unwind_drains_and_open_fails_with_full_refund` - the single happy
//!   path: zero-acked error -> drain both legs -> arrival -> `OpenFailed`
//!   with the downpayment refunded in full and the open-failed event.
//! - AC5/AC6 `partial_arrival_does_not_finish_then_both_complete` - the
//!   aggregate-by-currency arrival check: with only ONE drained currency
//!   landed the drain stays `Unwinding`; once BOTH land it finishes.
//! - AC7 `drain_transfer_out_timeout_reemits` - a drain transfer-out timeout
//!   re-emits the in-flight leg verbatim and the drain stays `Unwinding`.
//! - AC8 `drain_transfer_out_error_absorbed_until_heal` - a drain
//!   transfer-out error is absorbed (`remote-error`) and re-emitted only on a
//!   permissionless `Heal`.
//! - AC8 `unwind_over_nonzero_window_closes_lpp_loan` - across a non-zero
//!   drain window the LPP loan accrues interest; after the unwind the loan is
//!   CLOSED (`principal_due == 0` / loan absent), proving the reserve covered
//!   the interest and the full principal was repaid.

use currencies::Lpn;
use finance::{coin::Coin, duration::Duration};
use lease::api::query::{StateResponse, opening::OngoingTrx as OpeningOngoingTrx};
use remote_lease::callback::{RemoteErrorMessage, RemoteLeaseCallback, RemoteOperationOutcome};
use sdk::cosmwasm_std::{Addr, Event};

use crate::common::{
    self, USER,
    lpp::LppQueryMsg,
    remote_lease_controller_stub::{self as stub, ResponseMode, op_tag},
    test_case::TestCase,
};

use super::{DOWNPAYMENT, LeaseTestCase, LpnCurrency, PaymentCurrency, repay};

/// A non-zero drain window so the LPP loan accrues some interest before the
/// unwind closes it. A zero-length window would close the loan at exactly zero
/// interest, and the reserve's `CoverLiquidationLosses(0)` would attempt a
/// zero-amount bank send - the realistic on-chain drain always spans blocks
/// and accrues interest, so the tests model a non-trivial window.
const DRAIN_WINDOW: Duration = Duration::from_days(1);

const OPEN_UNWIND_EVENT: &str = "wasm-ls-open-unwind";
const OPEN_FAILED_EVENT: &str = "wasm-ls-remote-lease-open-failed";

// === AC4: single happy-path drain -> OpenFailed + full refund ===========

/// AC4: a zero-acked opening error drains the downpayment and the principal
/// home, and on full arrival the lease closes as `OpenFailed` with the whole
/// downpayment refunded to the customer and the open-failed event emitted.
#[test]
fn unwind_drains_and_open_fails_with_full_refund() {
    let (mut test_case, lease, controller) = start_opening_leg_one_in_flight();
    let customer = sdk::testing::user(USER);
    let downpayment_before = balance::<PaymentCurrency>(&test_case, &customer);
    // capture the drained principal while the oracle price is still fresh -
    // the time-shift below expires the feed
    let principal = super::quote_borrow(&test_case, DOWNPAYMENT);
    // the reserve covers the loan interest at close - fund it generously so
    // the cover always succeeds
    fund_reserve(&mut test_case);

    // default `Ok` mode: both drain legs ack inline and the drain settles at
    // the funds-arrival wait, still Unwinding.
    inject_opening_error(&mut test_case, &controller, &lease, "leg one under floor");
    assert_unwinding(&test_case, &lease);
    assert_eq!(
        2,
        stub::recorded_transfer_outs(&test_case.app, &controller, &lease).len(),
        "the drain must transfer out both the downpayment and the principal",
    );

    // the drain spans real time, so the loan accrues interest the reserve
    // covers at close
    test_case.app.time_shift(DRAIN_WINDOW);

    // land both drained currencies on the lease's local account and fire the
    // arrival poll
    credit_downpayment(&mut test_case, &lease);
    credit_principal(&mut test_case, &lease, principal);
    let arrival = repay::deliver_funds_arrival_alarm(&mut test_case, lease.clone());

    expect_event(&arrival.events, OPEN_FAILED_EVENT);
    assert_open_failed(&test_case, &lease);

    let downpayment_after = balance::<PaymentCurrency>(&test_case, &customer);
    assert_eq!(
        downpayment_before + DOWNPAYMENT,
        downpayment_after,
        "the whole downpayment must be refunded to the customer",
    );
}

// === AC5/AC6: aggregate-by-currency arrival check =======================

/// AC5 (load-bearing) + AC6: the drain finishes only once BOTH drained
/// currencies have landed. With only the downpayment credited the
/// funds-arrival poll keeps the drain `Unwinding` (the principal is still
/// outstanding); once the principal lands too the poll closes the lease as
/// `OpenFailed`.
///
/// The opening's two drained coins are DISTINCT currencies here - the
/// downpayment is `PaymentCurrency` (`Nls`) and the principal is
/// `LpnCurrency`. The same-currency case (a `Lpn` downpayment) is covered
/// directly by the `OpeningUnwindTask::all_received` unit tests
/// (`same_currency_requires_both_legs`); driving a same-currency *opening*
/// end-to-end is impractical because the shared opening helpers
/// (`try_init_lease`, `quote_borrow`, `DOWNPAYMENT`) are all keyed to
/// `PaymentCurrency = Nls`, so this E2E asserts the load-bearing
/// partial-arrival-does-not-finish property over the distinct-currency drain.
#[test]
fn partial_arrival_does_not_finish_then_both_complete() {
    let (mut test_case, lease, controller) = start_opening_leg_one_in_flight();
    let principal = super::quote_borrow(&test_case, DOWNPAYMENT);
    fund_reserve(&mut test_case);

    inject_opening_error(&mut test_case, &controller, &lease, "leg one under floor");
    assert_unwinding(&test_case, &lease);

    test_case.app.time_shift(DRAIN_WINDOW);

    // only the downpayment has landed: the aggregate check on the principal
    // currency is still short, so the drain must NOT finish
    credit_downpayment(&mut test_case, &lease);
    let _partial = repay::deliver_funds_arrival_alarm(&mut test_case, lease.clone());
    assert_unwinding(&test_case, &lease);

    // the principal lands too: now both currencies clear and the drain closes
    credit_principal(&mut test_case, &lease, principal);
    let arrival = repay::deliver_funds_arrival_alarm(&mut test_case, lease.clone());
    expect_event(&arrival.events, OPEN_FAILED_EVENT);
    assert_open_failed(&test_case, &lease);
}

// === AC7: a drain transfer-out timeout re-emits verbatim ================

/// AC7: a TIMEOUT on an in-flight drain transfer-out re-emits the leg
/// verbatim (a new recorded transfer-out, no advance) and the drain stays
/// `Unwinding` - the same timeout-re-emits rule the paid-close drain uses.
#[test]
fn drain_transfer_out_timeout_reemits() {
    let (mut test_case, lease, controller) = start_opening_leg_one_in_flight();

    // hold the first drain transfer-out in flight
    stub::set_response_mode(
        &mut test_case.app,
        &controller,
        op_tag::TRANSFER_OUT,
        ResponseMode::Delayed,
    );
    inject_opening_error(&mut test_case, &controller, &lease, "leg one under floor");
    assert_unwinding(&test_case, &lease);
    let transfers_before = stub::recorded_transfer_outs(&test_case.app, &controller, &lease).len();
    assert_eq!(1, transfers_before, "the first drain leg is in flight");

    // a timeout of the in-flight transfer re-emits it verbatim
    let reemit = stub::inject_callback(
        &mut test_case.app,
        &controller,
        &lease,
        RemoteLeaseCallback {
            nonce: 0,
            outcome: RemoteOperationOutcome::OperationTimeout,
        },
    );
    expect_attribute(&reemit.events, OPEN_UNWIND_EVENT, "timeout", "retry");

    assert_eq!(
        transfers_before + 1,
        stub::recorded_transfer_outs(&test_case.app, &controller, &lease).len(),
        "a drain transfer-out timeout must re-emit the in-flight leg",
    );
    assert_unwinding(&test_case, &lease);
}

// === AC8: a drain transfer-out error is absorbed until heal ==============

/// AC8: an ERROR ack of an in-flight drain transfer-out is absorbed
/// (`absorbed = remote-error` under `wasm-ls-open-unwind`) and is NOT
/// auto-retried; a permissionless `Heal` re-emits the in-flight leg
/// (`heal = re-emit`) and the drain stays `Unwinding` - the same
/// absorb-until-heal rule the paid-close drain uses.
#[test]
fn drain_transfer_out_error_absorbed_until_heal() {
    let (mut test_case, lease, controller) = start_opening_leg_one_in_flight();

    // error the first drain transfer-out
    let reason = RemoteErrorMessage::new("drain leg failed").expect("within length cap");
    stub::set_response_mode(
        &mut test_case.app,
        &controller,
        op_tag::TRANSFER_OUT,
        ResponseMode::Err(reason),
    );
    let entered = inject_opening_error(&mut test_case, &controller, &lease, "leg one under floor");

    // the errored transfer-out is absorbed, not retried
    expect_attribute(
        &entered.events,
        OPEN_UNWIND_EVENT,
        "absorbed",
        "remote-error",
    );
    assert_unwinding(&test_case, &lease);
    let transfers_after_absorb =
        stub::recorded_transfer_outs(&test_case.app, &controller, &lease).len();

    // a permissionless heal re-emits the in-flight transfer
    stub::set_response_mode(
        &mut test_case.app,
        &controller,
        op_tag::TRANSFER_OUT,
        ResponseMode::Ok,
    );
    let healed = super::heal(&mut test_case, lease.clone());
    expect_attribute(&healed.events, OPEN_UNWIND_EVENT, "heal", "re-emit");
    assert!(
        stub::recorded_transfer_outs(&test_case.app, &controller, &lease).len()
            > transfers_after_absorb,
        "the heal must re-emit the in-flight drain transfer-out",
    );
}

// === AC8: a non-zero-window unwind closes the LPP loan ==================

/// AC8: across a NON-ZERO drain window the LPP loan accrues interest; the
/// unwind's reserve-covered full close repays the loan IN FULL, so the loan
/// CLOSES (`Loan` query returns `None`). This proves the reserve covered the
/// accrued interest and the whole principal was repaid - a principal-only
/// repay would leave the loan open with the interest (and an equal principal)
/// outstanding.
#[test]
fn unwind_over_nonzero_window_closes_lpp_loan() {
    let (mut test_case, lease, controller) = start_opening_leg_one_in_flight();
    let principal = super::quote_borrow(&test_case, DOWNPAYMENT);
    fund_reserve(&mut test_case);

    inject_opening_error(&mut test_case, &controller, &lease, "leg one under floor");
    assert_unwinding(&test_case, &lease);

    // a NON-ZERO window so the loan accrues interest the reserve must cover
    test_case.app.time_shift(DRAIN_WINDOW);
    assert!(
        loan_of(&test_case, &lease).is_some(),
        "the LPP loan must still be open mid-drain",
    );
    // the drain is observable as Unwinding right up to the arrival close
    assert_unwinding(&test_case, &lease);

    credit_downpayment(&mut test_case, &lease);
    credit_principal(&mut test_case, &lease, principal);
    let _arrival = repay::deliver_funds_arrival_alarm(&mut test_case, lease.clone());

    assert_open_failed(&test_case, &lease);
    assert!(
        loan_of(&test_case, &lease).is_none(),
        "the LPP loan must be CLOSED after the reserve-covered full repay",
    );
}

// === drivers ============================================================

/// Open a lease and hold the opening swaps in flight via the `Delayed` SWAP
/// mode, with leg 1 (the downpayment leg, `total_out == 0`) outstanding -
/// the entry condition for the zero-acked clean unwind. The downpayment is in
/// `PaymentCurrency` (`Nls`) and the principal in `LpnCurrency`, so the two
/// drained coins are DISTINCT currencies.
fn start_opening_leg_one_in_flight() -> (LeaseTestCase, Addr, Addr) {
    let mut test_case = super::create_test_case::<PaymentCurrency>();
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

/// Inject a deterministic zero-acked `OperationErr` carrying the lease's
/// current in-flight swap nonce (the stand-in stamps the last recorded nonce).
fn inject_opening_error(
    test_case: &mut LeaseTestCase,
    controller: &Addr,
    lease: &Addr,
    reason: &str,
) -> sdk::cw_multi_test::AppResponse {
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

// === query / driver helpers =============================================

/// The Lpn the reserve is pre-funded with so its `CoverLiquidationLosses`
/// covering the drain-window interest never hits `InsufficientBalance`. Sized
/// well above any interest a `DRAIN_WINDOW` accrues on the test principal.
const RESERVE_FUNDING: Coin<LpnCurrency> = Coin::new(1_000_000_000_000);

/// Pre-fund the reserve contract so its interest cover at close succeeds.
fn fund_reserve(test_case: &mut LeaseTestCase) {
    let reserve = test_case.address_book.reserve().clone();
    test_case.send_funds_from_admin(reserve, &[common::cwcoin::<LpnCurrency>(RESERVE_FUNDING)]);
}

/// Credit the drained downpayment (`PaymentCurrency`) onto the lease's local
/// account, standing in for its ICS-20 arrival home.
fn credit_downpayment(test_case: &mut LeaseTestCase, lease: &Addr) {
    test_case.send_funds_from_admin(
        lease.clone(),
        &[common::cwcoin::<PaymentCurrency>(DOWNPAYMENT)],
    );
}

/// Credit the drained loan principal (`LpnCurrency`) onto the lease's local
/// account, standing in for its ICS-20 arrival home.
fn credit_principal(test_case: &mut LeaseTestCase, lease: &Addr, principal: Coin<LpnCurrency>) {
    test_case.send_funds_from_admin(lease.clone(), &[common::cwcoin::<LpnCurrency>(principal)]);
}

fn balance<C>(test_case: &LeaseTestCase, account: &Addr) -> Coin<C>
where
    C: currency::CurrencyDef,
{
    use platform::bank::{self, BankAccountView};
    bank::account_view(account, test_case.app.query())
        .balance::<C>()
        .expect("balance query must succeed")
}

/// The LPP loan recorded for the lease, or `None` once the loan is closed.
fn loan_of(test_case: &LeaseTestCase, lease: &Addr) -> lpp::msg::QueryLoanResponse<LpnCurrency> {
    test_case
        .app
        .query()
        .query_wasm_smart(
            test_case.address_book.lpp().clone(),
            &LppQueryMsg::Loan {
                lease_addr: lease.clone(),
            },
        )
        .expect("the LPP loan query must succeed")
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

#[track_caller]
fn assert_unwinding(test_case: &LeaseTestCase, lease: &Addr) {
    match super::state_query(test_case, lease.clone()) {
        StateResponse::Opening {
            in_progress: OpeningOngoingTrx::Unwinding,
            ..
        } => (),
        other => panic!("expected the opening unwind drain, got {other:?}"),
    }
}

#[track_caller]
fn assert_open_failed(test_case: &LeaseTestCase, lease: &Addr) {
    match super::state_query(test_case, lease.clone()) {
        StateResponse::OpenFailed { .. } => (),
        other => panic!("expected StateResponse::OpenFailed, got {other:?}"),
    }
}

#[track_caller]
fn expect_event(events: &[Event], event_type: &str) {
    assert!(
        events.iter().any(|event| event.ty == event_type),
        "expected event `{event_type}`, got {events:?}",
    );
}

#[track_caller]
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

// Pull `Lpn` into the build graph so its currency definitions are loaded.
#[allow(dead_code)]
fn _lpn_anchor() -> currency::CurrencyDTO<currencies::Lpns> {
    currency::dto::<Lpn, _>()
}
