//! Swap E2E (issue nolus-protocol/ibc-solray#142, opening swap leg).
//!
//! `Operation::Swap` is single-coin per call. When the opening holds
//! multiple currencies to swap, the lease emits sequential single-coin
//! `Swap` calls and decrements `acks_left` per acknowledgment. There is
//! no batched multi-leg packet.
//!
//! Documented drivers:
//!
//! - `swap_single_coin_happy_path` — every emitted `ExecuteMsg::Swap`
//!   carries a single coin-in/min-out pair; the stand-in's
//!   `OperationResponse::Swap { amount_out = min_out }` acks drive the
//!   opening to `Opened` with the sum of the floors.
//! - `swap_multi_currency_sequential_acks` — two currencies needing a
//!   swap produce two sequential `Swap` calls with the `acks_left`
//!   countdown observable via `OngoingTrx` between the acks.
//! - `swap_delayed_ack_visible_in_query` — with `ResponseMode::Delayed`
//!   the in-flight leg stays observable across block advances.
//! - `opening_swap_pins_the_opening_bound` — a tightened
//!   `MaxSlippages::opening` (below the unchanged liquidation bound) makes the
//!   opening leg's floor match the opening bound, proving the swap reads
//!   `opening`, not `liquidation`.
//!
//! Hardening drivers:
//!
//! - `downpayment_in_asset_currency_swaps_once` — a downpayment already
//!   in the lease currency folds in without a swap; only the borrow leg
//!   goes out.
//! - `underpaid_leg_retries_in_place` — an acknowledgment below the
//!   leg's floor re-emits only that leg (`anomaly = under-min-out`);
//!   the retried leg settles and the opening completes.
//! - `swap_ack_in_transfer_leg_absorbed` — a `Swap` success delivered
//!   while the machine sits in the ICA transfer-out leg is absorbed with
//!   an event, the countdown does not advance, and the opening still
//!   completes.
//! - `wrong_variant_callback_absorbed_then_heal_recovers` — a decodable
//!   but non-swap success payload is absorbed
//!   (`unexpected-response-variant`); `ExecuteMsg::Heal` re-emits the
//!   in-flight leg and the opening completes.
//! - `out_of_registry_ticker_absorbed_at_lease` — a success ack naming a
//!   ticker outside the currency registry passes the controller
//!   wire-shaped and is absorbed at the lease (`undecodable-response`).
//! - `duplicate_ack_not_miscredited` — replaying a leg's superseded nonce
//!   after the sequence advances is absorbed (`nonce-mismatch`); no second
//!   leg is mis-credited (#636).
//! - `heal_race_original_ack_absorbed` — a `Heal()` re-emits with a fresh
//!   nonce; the original packet's late ack (old nonce) is absorbed while the
//!   healed re-emission's ack (new nonce) credits exactly once (#636).
//! - `opening_swap_out_currency_mismatch_absorbed` — a success ack in a valid
//!   lease-asset currency that is not the opening's output currency trips the
//!   deliver-ack currency guard and is absorbed (`out-currency-mismatch`); the
//!   leg countdown does not advance (round-2 #644 backfill).
//! - `opening_swap_stale_nonce_timeout_absorbed` — a timeout callback carrying a
//!   superseded nonce is absorbed (`nonce-mismatch`) before the timeout arm, so
//!   the in-flight leg is neither re-emitted nor advanced (round-2 #644 backfill).

use crate::common::testing;
use currencies::{PaymentGroup, testing::LeaseC1};
use currency::{CurrencyDef, MemberOf};
use dex::MaxSlippage;
use finance::{
    coin::{Coin, CoinDTO},
    duration::Duration,
    percent::Percent100,
    price,
};
use lease::api::{
    ExecuteMsg,
    query::{StateResponse, opening::OngoingTrx as OpeningOngoingTrx},
};
use remote_lease::{
    callback::{RemoteLeaseCallback, RemoteOperationOutcome},
    response::{
        OperationResponse, SwapResponse, Ticker, TransferOutResponse, WireCoin,
        WireOperationResponse, WireSwapResponse,
    },
};
use sdk::{
    cosmwasm_std::{Addr, Event},
    cw_multi_test::AppResponse,
};

use crate::common::{
    self, ADMIN, LEASE_ADMIN,
    remote_lease_controller_stub::{self as stub, ResponseMode, op_tag},
    test_case::TestCase,
};

use super::{
    DOWNPAYMENT, LeaseCoin, LeaseCurrency, LeaseTestCase, LpnCoin, LpnCurrency, PaymentCurrency,
};

const OPENING_SWAP_EVENT: &str = "wasm-ls-open-swap";
const ABSORB_EVENT: &str = "wasm-remote-callback";

#[test]
fn swap_single_coin_happy_path() {
    let (mut test_case, lease, controller) = start_open(DOWNPAYMENT);
    let exp_borrow = borrow_quote(&test_case, DOWNPAYMENT);

    let _response = transfer_funds(&mut test_case, &lease, DOWNPAYMENT);

    let swaps = stub::recorded_swaps(&test_case.app, &controller, &lease);
    assert_eq!(2, swaps.len());
    assert_eq!(
        &CoinDTO::<PaymentGroup>::from(DOWNPAYMENT),
        swaps[0].coin_in()
    );
    assert_eq!(
        &CoinDTO::<PaymentGroup>::from(exp_borrow),
        swaps[1].coin_in()
    );
    swaps.iter().for_each(|params| {
        assert_eq!(
            currency::dto::<LeaseCurrency, PaymentGroup>(),
            params.min_out().currency()
        );
    });

    let total_floor: LeaseCoin =
        LeaseCoin::new(swaps.iter().map(|params| params.min_out().amount()).sum());
    assert_eq!(total_floor, opened_amount(&test_case, lease));
}

#[test]
fn swap_multi_currency_sequential_acks() {
    let (mut test_case, lease, controller) = start_open(DOWNPAYMENT);
    stub::set_response_mode(
        &mut test_case.app,
        &controller,
        op_tag::SWAP,
        ResponseMode::Delayed,
    );

    let _response = transfer_funds(&mut test_case, &lease, DOWNPAYMENT);
    assert_eq!(2, opening_acks_left(&test_case, lease.clone()));
    assert_eq!(
        1,
        stub::recorded_swaps(&test_case.app, &controller, &lease).len()
    );

    let _delivery = stub::deliver_pending_callback(&mut test_case.app, &controller, op_tag::SWAP);
    assert_eq!(1, opening_acks_left(&test_case, lease.clone()));
    assert_eq!(
        2,
        stub::recorded_swaps(&test_case.app, &controller, &lease).len()
    );

    let _delivery = stub::deliver_pending_callback(&mut test_case.app, &controller, op_tag::SWAP);
    let _amount = opened_amount(&test_case, lease);
}

#[test]
fn swap_delayed_ack_visible_in_query() {
    let (mut test_case, lease, controller) = start_open(DOWNPAYMENT);
    stub::set_response_mode(
        &mut test_case.app,
        &controller,
        op_tag::SWAP,
        ResponseMode::Delayed,
    );

    let _response = transfer_funds(&mut test_case, &lease, DOWNPAYMENT);

    test_case.app.time_shift(Duration::from_secs(6));
    assert_eq!(2, opening_acks_left(&test_case, lease.clone()));

    test_case.app.time_shift(Duration::from_secs(6));
    assert_eq!(2, opening_acks_left(&test_case, lease.clone()));

    let _delivery = stub::deliver_pending_callback(&mut test_case.app, &controller, op_tag::SWAP);
    test_case.app.time_shift(Duration::from_secs(6));
    assert_eq!(1, opening_acks_left(&test_case, lease.clone()));

    let _delivery = stub::deliver_pending_callback(&mut test_case.app, &controller, op_tag::SWAP);
    let _amount = opened_amount(&test_case, lease);
}

#[test]
fn downpayment_in_asset_currency_swaps_once() {
    let downpayment = LeaseCoin::new(10_000);
    let (mut test_case, lease, controller) = start_open(downpayment);
    let exp_borrow = borrow_quote(&test_case, downpayment);

    let _response = transfer_funds(&mut test_case, &lease, downpayment);

    let swaps = stub::recorded_swaps(&test_case.app, &controller, &lease);
    assert_eq!(1, swaps.len());
    assert_eq!(
        &CoinDTO::<PaymentGroup>::from(exp_borrow),
        swaps[0].coin_in()
    );

    let folded_plus_floor = downpayment + LeaseCoin::new(swaps[0].min_out().amount());
    assert_eq!(folded_plus_floor, opened_amount(&test_case, lease));
}

#[test]
fn underpaid_leg_retries_in_place() {
    let (mut test_case, lease, controller) = start_open(DOWNPAYMENT);
    stub::set_response_mode(
        &mut test_case.app,
        &controller,
        op_tag::SWAP,
        ResponseMode::UnderpayingOnce,
    );

    let response = transfer_funds(&mut test_case, &lease, DOWNPAYMENT);
    expect_attribute(
        &response.events,
        OPENING_SWAP_EVENT,
        "anomaly",
        "under-min-out",
    );

    // the first leg goes out twice - the underpaid attempt and its
    // retry - the second leg once; the retry repeats the pinned floor
    let swaps = stub::recorded_swaps(&test_case.app, &controller, &lease);
    assert_eq!(3, swaps.len());
    assert_eq!(swaps[0].coin_in(), swaps[1].coin_in());
    assert_eq!(swaps[0].min_out(), swaps[1].min_out());
    assert_eq!(
        &CoinDTO::<PaymentGroup>::from(DOWNPAYMENT),
        swaps[0].coin_in()
    );

    let total_floor: LeaseCoin =
        LeaseCoin::new(swaps[1].min_out().amount() + swaps[2].min_out().amount());
    assert_eq!(total_floor, opened_amount(&test_case, lease));
}

#[test]
fn swap_ack_in_transfer_leg_absorbed() {
    let (mut test_case, lease, controller) = start_open(DOWNPAYMENT);

    // The opening sits in the funding leg - the downpayment transfer is in
    // flight. A swap acknowledgment arriving now must neither error nor
    // advance the funding leg; only the swap leg credits a remote callback.
    assert_transfers_pending(&test_case, lease.clone());

    let unexpected = RemoteLeaseCallback {
        nonce: 0,
        outcome: RemoteOperationOutcome::OperationOk(
            OperationResponse::Swap(SwapResponse {
                amount_out: Coin::<LeaseCurrency>::new(1).into(),
            })
            .into(),
        ),
    };
    let injected = stub::inject_callback(&mut test_case.app, &controller, &lease, unexpected);
    expect_attribute(&injected.events, ABSORB_EVENT, "absorbed", "response");
    assert_transfers_pending(&test_case, lease.clone());

    let _response = transfer_funds(&mut test_case, &lease, DOWNPAYMENT);

    let _amount = opened_amount(&test_case, lease);
}

#[test]
fn wrong_variant_callback_absorbed_then_heal_recovers() {
    let (mut test_case, lease, controller) = start_open(DOWNPAYMENT);
    stub::set_response_mode(
        &mut test_case.app,
        &controller,
        op_tag::SWAP,
        ResponseMode::Delayed,
    );

    let _response = transfer_funds(&mut test_case, &lease, DOWNPAYMENT);
    assert_eq!(2, opening_acks_left(&test_case, lease.clone()));

    let wrong_variant = RemoteLeaseCallback {
        nonce: 0,
        outcome: RemoteOperationOutcome::OperationOk(WireOperationResponse::TransferOut(
            TransferOutResponse {},
        )),
    };
    let injected = stub::inject_callback(&mut test_case.app, &controller, &lease, wrong_variant);
    expect_attribute(
        &injected.events,
        OPENING_SWAP_EVENT,
        "absorbed",
        "unexpected-response-variant",
    );
    assert_eq!(2, opening_acks_left(&test_case, lease.clone()));

    stub::set_response_mode(
        &mut test_case.app,
        &controller,
        op_tag::SWAP,
        ResponseMode::Ok,
    );
    let healed = test_case
        .app
        .execute(
            testing::user(ADMIN),
            lease.clone(),
            &ExecuteMsg::Heal(),
            &[],
        )
        .expect("heal must re-emit the in-flight leg")
        .unwrap_response();
    expect_attribute(&healed.events, OPENING_SWAP_EVENT, "heal", "re-emit");

    let _amount = opened_amount(&test_case, lease);
}

// AC (#636): a stale/duplicate ack must not be miscredited. The first leg's
// ack credits normally and the sequence advances; replaying that leg's (now
// superseded) nonce after the advance is absorbed with `nonce-mismatch` - no
// second leg is mis-credited, the countdown and the recorded-swap set are
// unchanged.
#[test]
fn duplicate_ack_not_miscredited() {
    let (mut test_case, lease, controller) = start_open(DOWNPAYMENT);
    stub::set_response_mode(
        &mut test_case.app,
        &controller,
        op_tag::SWAP,
        ResponseMode::Delayed,
    );

    let _response = transfer_funds(&mut test_case, &lease, DOWNPAYMENT);
    assert_eq!(2, opening_acks_left(&test_case, lease.clone()));

    // the first leg's ack credits normally and advances to the second leg
    let _delivery = stub::deliver_pending_callback(&mut test_case.app, &controller, op_tag::SWAP);
    assert_eq!(1, opening_acks_left(&test_case, lease.clone()));

    let swaps_before = stub::recorded_swaps(&test_case.app, &controller, &lease).len();
    let nonces = stub::recorded_swap_nonces(&test_case.app, &controller, &lease);
    let stale_nonce = nonces[0];

    // replay the first leg's now-superseded nonce with an otherwise-creditable
    // amount: only the nonce check stops it from advancing the second leg
    let duplicate = RemoteOperationOutcome::OperationOk(
        OperationResponse::Swap(SwapResponse {
            amount_out: Coin::<LeaseCurrency>::new(1_000).into(),
        })
        .into(),
    );
    let injected = stub::inject_callback_with_nonce(
        &mut test_case.app,
        &controller,
        &lease,
        stale_nonce,
        duplicate,
    );
    expect_attribute(
        &injected.events,
        OPENING_SWAP_EVENT,
        "absorbed",
        "nonce-mismatch",
    );

    // no second leg mis-credited: the countdown and the recorded-swap set are
    // untouched by the stale duplicate
    assert_eq!(1, opening_acks_left(&test_case, lease.clone()));
    assert_eq!(
        swaps_before,
        stub::recorded_swaps(&test_case.app, &controller, &lease).len()
    );
}

// AC (#636) - core race: an operator Heal() re-emits the in-flight leg with a
// fresh nonce. The ORIGINAL packet's late ack (old nonce) is then absorbed
// while the healed re-emission's ack (new nonce) is credited - proving no
// double-credit when a heal races a still-resolvable original packet.
#[test]
fn heal_race_original_ack_absorbed() {
    let (mut test_case, lease, controller) = start_open(DOWNPAYMENT);
    stub::set_response_mode(
        &mut test_case.app,
        &controller,
        op_tag::SWAP,
        ResponseMode::Delayed,
    );

    let _response = transfer_funds(&mut test_case, &lease, DOWNPAYMENT);
    assert_eq!(2, opening_acks_left(&test_case, lease.clone()));
    let original_nonce = stub::recorded_swap_nonces(&test_case.app, &controller, &lease)[0];

    // the operator heals the still-in-flight leg, re-emitting it with a fresh
    // nonce
    let healed = test_case
        .app
        .execute(
            testing::user(ADMIN),
            lease.clone(),
            &ExecuteMsg::Heal(),
            &[],
        )
        .expect("heal must re-emit the in-flight leg")
        .unwrap_response();
    expect_attribute(&healed.events, OPENING_SWAP_EVENT, "heal", "re-emit");

    let nonces = stub::recorded_swap_nonces(&test_case.app, &controller, &lease);
    let healed_nonce = nonces[1];
    assert!(
        original_nonce < healed_nonce,
        "heal must re-emit with a strictly greater nonce"
    );

    // the original packet's late ack carries the now-stale nonce → absorbed
    let stale = RemoteOperationOutcome::OperationOk(
        OperationResponse::Swap(SwapResponse {
            amount_out: Coin::<LeaseCurrency>::new(1_000).into(),
        })
        .into(),
    );
    let injected = stub::inject_callback_with_nonce(
        &mut test_case.app,
        &controller,
        &lease,
        original_nonce,
        stale,
    );
    expect_attribute(
        &injected.events,
        OPENING_SWAP_EVENT,
        "absorbed",
        "nonce-mismatch",
    );
    assert_eq!(2, opening_acks_left(&test_case, lease.clone()));

    // the healed re-emission's ack (fresh nonce) credits exactly once. The
    // amount must clear the healed leg's pinned floor (`min_out`), or production
    // would treat it as under-min-out and re-emit instead of crediting - the
    // amount a real counterparty pays is always at or above that floor.
    let healed_floor = stub::recorded_swaps(&test_case.app, &controller, &lease)
        .last()
        .expect("the healed re-emission was recorded")
        .min_out()
        .amount();
    let fresh = RemoteOperationOutcome::OperationOk(
        OperationResponse::Swap(SwapResponse {
            amount_out: Coin::<LeaseCurrency>::new(healed_floor).into(),
        })
        .into(),
    );
    let _credited = stub::inject_callback_with_nonce(
        &mut test_case.app,
        &controller,
        &lease,
        healed_nonce,
        fresh,
    );
    assert_eq!(1, opening_acks_left(&test_case, lease.clone()));
}

// The controller forwards success acks wire-shaped (issue #637): a ticker
// outside the currency registry reaches the lease, whose typed decode fails
// and absorbs the callback - the controller's ack tx commits.
#[test]
fn out_of_registry_ticker_absorbed_at_lease() {
    let (mut test_case, lease, controller) = start_open(DOWNPAYMENT);
    stub::set_response_mode(
        &mut test_case.app,
        &controller,
        op_tag::SWAP,
        ResponseMode::Delayed,
    );

    let _response = transfer_funds(&mut test_case, &lease, DOWNPAYMENT);
    assert_eq!(2, opening_acks_left(&test_case, lease.clone()));

    let alien_ticker = RemoteLeaseCallback {
        nonce: 0,
        outcome: RemoteOperationOutcome::OperationOk(WireOperationResponse::Swap(
            WireSwapResponse {
                amount_out: WireCoin::new(42, Ticker::new("NOT_IN_REGISTRY")),
            },
        )),
    };
    let injected = stub::inject_callback(&mut test_case.app, &controller, &lease, alien_ticker);
    expect_attribute(
        &injected.events,
        OPENING_SWAP_EVENT,
        "absorbed",
        "undecodable-response",
    );
    assert_eq!(2, opening_acks_left(&test_case, lease.clone()));
}

// The opening swap must bound itself by `MaxSlippages::opening`, not the
// liquidation bound it borrowed before #639. Tighten only `opening` (leaving
// liquidation at the fixture default), open, and assert the emitted opening
// leg pins the opening-bound floor — a value the looser liquidation bound
// could not produce.
#[test]
fn opening_swap_pins_the_opening_bound() {
    const OPENING_SLIPPAGE: Percent100 = Percent100::from_permille(50);

    let mut test_case = super::create_test_case::<PaymentCurrency>();
    set_opening_slippage(&mut test_case, OPENING_SLIPPAGE);

    let lease = super::try_init_lease(&mut test_case, DOWNPAYMENT, None);
    let controller = test_case.address_book.remote_lease_controller().clone();
    let exp_borrow = borrow_quote(&test_case, DOWNPAYMENT);

    let _response = transfer_funds(&mut test_case, &lease, DOWNPAYMENT);

    let swaps = stub::recorded_swaps(&test_case.app, &controller, &lease);
    assert_eq!(2, swaps.len());
    let borrow_quote_in_asset =
        price::total(exp_borrow, super::price_lpn_of::<LeaseCurrency>().inv()).unwrap();
    let opening_floor = CoinDTO::<PaymentGroup>::from(
        MaxSlippage::unchecked(OPENING_SLIPPAGE).min_out(borrow_quote_in_asset),
    );
    let liquidation_floor =
        CoinDTO::<PaymentGroup>::from(super::swap_min_out(borrow_quote_in_asset));

    assert_eq!(&opening_floor, swaps[1].min_out());
    assert_ne!(&liquidation_floor, swaps[1].min_out());
}

// A decodable success ack whose output is a valid lease asset but not the
// opening's output currency is absorbed at deliver-ack's currency guard, before
// the amount is inspected. The behaviour already ships; this closes the arm's
// integration gap.
#[test]
fn opening_swap_out_currency_mismatch_absorbed() {
    const MISMATCHED_OUTPUT: u128 = 1_000_000;

    let (mut test_case, lease, controller) = start_open(DOWNPAYMENT);
    stub::set_response_mode(
        &mut test_case.app,
        &controller,
        op_tag::SWAP,
        ResponseMode::Delayed,
    );
    // Consumed when the first swap leg is emitted, so its pending ack pays a
    // lease-asset currency (LeaseC1) other than the opening's output (LeaseC2).
    stub::set_next_swap_output(
        &mut test_case.app,
        &controller,
        Coin::<LeaseC1>::new(MISMATCHED_OUTPUT).into(),
    );

    let _response = transfer_funds(&mut test_case, &lease, DOWNPAYMENT);
    assert_eq!(2, opening_acks_left(&test_case, lease.clone()));

    let absorbed = stub::deliver_pending_callback(&mut test_case.app, &controller, op_tag::SWAP);
    expect_attribute(
        &absorbed.events,
        OPENING_SWAP_EVENT,
        "absorbed",
        "out-currency-mismatch",
    );
    assert_eq!(2, opening_acks_left(&test_case, lease.clone()));
}

// A timeout callback carrying a superseded nonce is rejected by the nonce gate
// before the timeout retry arm, so the in-flight leg is neither re-emitted nor
// advanced. The behaviour already ships; this closes the arm's integration gap.
#[test]
fn opening_swap_stale_nonce_timeout_absorbed() {
    let (mut test_case, lease, controller) = start_open(DOWNPAYMENT);
    stub::set_response_mode(
        &mut test_case.app,
        &controller,
        op_tag::SWAP,
        ResponseMode::Delayed,
    );

    let _response = transfer_funds(&mut test_case, &lease, DOWNPAYMENT);
    assert_eq!(2, opening_acks_left(&test_case, lease.clone()));

    let in_flight = stub::recorded_swap_nonces(&test_case.app, &controller, &lease)
        .last()
        .copied()
        .expect("the in-flight swap leg recorded a nonce");
    let stale = in_flight - 1;
    let absorbed = stub::inject_callback_with_nonce(
        &mut test_case.app,
        &controller,
        &lease,
        stale,
        RemoteOperationOutcome::OperationTimeout,
    );
    expect_attribute(
        &absorbed.events,
        OPENING_SWAP_EVENT,
        "absorbed",
        "nonce-mismatch",
    );
    assert_eq!(2, opening_acks_left(&test_case, lease.clone()));
}

fn set_opening_slippage(test_case: &mut LeaseTestCase, opening: Percent100) {
    let leaser_addr = test_case.address_book.leaser().clone();
    let mut new_config = common::leaser::Instantiator::new_config();
    new_config.lease_max_slippages.opening = MaxSlippage::unchecked(opening);
    let _response = test_case
        .app
        .execute(
            testing::user(LEASE_ADMIN),
            leaser_addr,
            &leaser::msg::ExecuteMsg::ConfigLeases(new_config),
            &[],
        )
        .unwrap();
}

fn start_open<DownpaymentC>(downpayment: Coin<DownpaymentC>) -> (LeaseTestCase, Addr, Addr)
where
    DownpaymentC: CurrencyDef,
{
    let mut test_case = super::create_test_case::<DownpaymentC>();
    let lease = super::try_init_lease(&mut test_case, downpayment, None);
    let controller = test_case.address_book.remote_lease_controller().clone();
    (test_case, lease, controller)
}

fn borrow_quote<DownpaymentC>(test_case: &LeaseTestCase, downpayment: Coin<DownpaymentC>) -> LpnCoin
where
    DownpaymentC: CurrencyDef,
    DownpaymentC::Group: MemberOf<PaymentGroup>,
{
    common::leaser::query_quote::<DownpaymentC, LeaseCurrency>(
        &test_case.app,
        test_case.address_book.leaser().clone(),
        downpayment,
        None,
    )
    .borrow
    .try_into()
    .unwrap()
}

fn transfer_funds<DownpaymentC>(
    test_case: &mut LeaseTestCase,
    lease: &Addr,
    downpayment: Coin<DownpaymentC>,
) -> AppResponse
where
    DownpaymentC: CurrencyDef,
    DownpaymentC::Group: MemberOf<PaymentGroup>,
{
    let exp_borrow = borrow_quote(test_case, downpayment);
    let ica_addr = TestCase::ica_addr(lease, TestCase::LEASE_ICA_ID);

    common::lease::fund_remote_lease::<DownpaymentC, LpnCurrency>(
        &mut test_case.app,
        lease.clone(),
        ica_addr,
        (downpayment, exp_borrow),
    )
    .unwrap_response()
}

fn opening_acks_left(test_case: &LeaseTestCase, lease: Addr) -> u8 {
    match super::state_query(test_case, lease) {
        StateResponse::Opening {
            in_progress: OpeningOngoingTrx::BuyAsset { acks_left },
            ..
        } => acks_left,
        other => panic!("expected the in-flight swap leg, got {other:?}"),
    }
}

fn opened_amount(test_case: &LeaseTestCase, lease: Addr) -> LeaseCoin {
    match super::state_query(test_case, lease) {
        StateResponse::Opened { amount, .. } => amount.try_into().unwrap(),
        other => panic!("expected an opened lease, got {other:?}"),
    }
}

fn assert_transfers_pending(test_case: &LeaseTestCase, lease: Addr) {
    match super::state_query(test_case, lease) {
        StateResponse::Opening {
            in_progress: OpeningOngoingTrx::Funding { .. },
            ..
        } => (),
        other => panic!("expected the funding leg, got {other:?}"),
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
