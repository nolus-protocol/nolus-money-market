//! Swap E2E for the remote-lease transport (issue #142 Phase 4).
//!
//! Post-refactor `SwapExactIn` folds the swap-input coins into a single
//! `remote_lease::swap::SwapParams` (`One` for a lone input currency, `Two` for
//! two) and emits ONE `ExecuteMsg::Swap { params, timeout }` to the controller.
//! There is no per-currency sequential call and no `acks_left` countdown — the
//! shipped design batches; a single `OperationResponse::Swap` ack drives the
//! transition. (The pre-refactor "single-coin per call" amendment these tests
//! were sketched against never shipped.)
//!
//! The opening buy-asset swap is the natural driver:
//! - a same-currency downpayment (== the lease asset) is excluded from the
//!   swap, leaving a single-input (`One`) loan swap;
//! - a payment-currency downpayment leaves a two-input (`Two`) swap of
//!   downpayment + loan.
//!
//! Coverage:
//! - `swap_single_coin_happy_path` — one `One`-variant `Swap` is emitted with
//!   the loan as coin-in and a positive min-out; the ack drives the transition.
//! - `swap_multi_currency_single_call` — a two-currency position emits one
//!   `Two`-variant `Swap`; a single ack transitions (no countdown).
//! - `swap_delayed_ack_visible_in_query` — with `ResponseMode::Delayed` the
//!   in-flight swap is observable via `OngoingTrx` across blocks until the ack
//!   is delivered.
//! - `swap_folds_same_currency_downpayment_into_output` — a same-currency
//!   downpayment is excluded from the swap, yet the opened lease's asset is the
//!   full position, proving the non-swapped coin is re-added to the swap output.

use currencies::PaymentGroup;
use currency::{CurrencyDef, MemberOf};
use finance::{
    coin::{Coin, CoinDTO},
    duration::Duration,
    fraction::Unit,
};
use lease::api::query::{StateResponse, opened::Status, opening::OngoingTrx as OpeningOngoingTrx};
use remote_lease::swap::SwapParams;
use sdk::cosmwasm_std::Addr;

use crate::common::{
    self,
    remote_lease_controller_stub::{self as stub, ResponseMode, SwapFill, op_tag},
    swap,
};

use super::{LeaseCoin, LeaseCurrency, LeaseTestCase, LpnCurrency, PaymentCurrency};

#[test]
fn swap_single_coin_happy_path() {
    let mut test_case = super::create_test_case::<LeaseCurrency>();
    // A same-currency downpayment is already the asset, so only the loan is
    // swapped — a single-input swap.
    let downpayment = LeaseCoin::new(10_000);
    let (lease, controller, exp_borrow) = drive_to_buy_asset_swap(&mut test_case, downpayment);

    let captured = swap::captured(&test_case.app, &controller);
    match captured {
        SwapParams::One { coin_in, min_out } => {
            assert_eq!(Into::<CoinDTO<PaymentGroup>>::into(exp_borrow), coin_in);
            assert!(
                !min_out.is_zero(),
                "the slippage min-out floor must be positive"
            );
        }
        other => panic!("expected a single-coin (`One`) swap, got {other:?}"),
    }
    assert!(matches!(
        super::state_query(&test_case, lease.clone()),
        StateResponse::Opening {
            in_progress: OpeningOngoingTrx::BuyAsset { .. },
            ..
        }
    ));

    // The ack drives the transition to the next state.
    deliver_open_swap(&mut test_case, &controller);
    let final_state = super::state_query(&test_case, lease);
    assert!(
        matches!(
            final_state,
            StateResponse::Opened {
                status: Status::Idle,
                ..
            }
        ),
        "the swap ack must cleanly open the lease, got {final_state:?}"
    );
}

#[test]
fn swap_folds_same_currency_downpayment_into_output() {
    let mut test_case = super::create_test_case::<LeaseCurrency>();
    // The downpayment is already the asset currency, so it is excluded from the
    // swap and only the loan is swapped. The opened lease's asset must be the
    // full position — the non-swapped downpayment re-added to the swap output.
    let downpayment = LeaseCoin::new(10_000);
    let (lease, controller, exp_borrow) = drive_to_buy_asset_swap(&mut test_case, downpayment);

    deliver_open_swap(&mut test_case, &controller);

    let final_state = super::state_query(&test_case, lease);
    let StateResponse::Opened { amount, status, .. } = final_state else {
        panic!("the swap ack must open the lease, got {final_state:?}");
    };
    assert_eq!(Status::Idle, status);
    assert_eq!(
        downpayment
            .to_primitive()
            .checked_add(exp_borrow.to_primitive())
            .expect("the test position must not overflow"),
        amount.amount(),
        "the same-currency downpayment must be folded into the swap output",
    );
}

#[test]
fn swap_multi_currency_single_call() {
    let mut test_case = super::create_test_case::<PaymentCurrency>();
    // A payment-currency downpayment is swapped alongside the loan — a
    // two-input swap batched into ONE call (not two sequential calls).
    let downpayment = super::DOWNPAYMENT;
    let (lease, controller, exp_borrow) = drive_to_buy_asset_swap(&mut test_case, downpayment);

    let captured = swap::captured(&test_case.app, &controller);
    match captured {
        SwapParams::Two {
            coin_in_1,
            coin_in_2,
            min_out,
        } => {
            assert_eq!(Into::<CoinDTO<PaymentGroup>>::into(downpayment), coin_in_1);
            assert_eq!(Into::<CoinDTO<PaymentGroup>>::into(exp_borrow), coin_in_2);
            assert!(
                !min_out.is_zero(),
                "the slippage min-out floor must be positive"
            );
        }
        other => panic!("expected a two-coin (`Two`) batched swap, got {other:?}"),
    }
    assert_eq!(
        1,
        swap::count(&test_case.app, &controller),
        "a multi-currency swap must be ONE batched call, not two sequential calls",
    );
    assert!(matches!(
        super::state_query(&test_case, lease.clone()),
        StateResponse::Opening {
            in_progress: OpeningOngoingTrx::BuyAsset { .. },
            ..
        }
    ));

    // A single ack transitions the lease — there is no per-currency countdown.
    deliver_open_swap(&mut test_case, &controller);
    let final_state = super::state_query(&test_case, lease);
    assert!(
        matches!(
            final_state,
            StateResponse::Opened {
                status: Status::Idle,
                ..
            }
        ),
        "the swap ack must cleanly open the lease, got {final_state:?}"
    );
}

#[test]
fn swap_delayed_ack_visible_in_query() {
    let mut test_case = super::create_test_case::<LeaseCurrency>();
    let downpayment = LeaseCoin::new(10_000);
    let (lease, controller, _exp_borrow) = drive_to_buy_asset_swap(&mut test_case, downpayment);

    // The swap ack is held (Delayed); the in-flight swap stays observable while
    // blocks advance.
    for _ in 0..2 {
        assert!(
            matches!(
                super::state_query(&test_case, lease.clone()),
                StateResponse::Opening {
                    in_progress: OpeningOngoingTrx::BuyAsset { .. },
                    ..
                }
            ),
            "the in-flight swap must remain visible while its ack is pending",
        );
        test_case.app.time_shift(Duration::from_secs(5));
    }

    // Delivering the delayed ack advances the lease.
    deliver_open_swap(&mut test_case, &controller);
    let final_state = super::state_query(&test_case, lease);
    assert!(
        matches!(
            final_state,
            StateResponse::Opened {
                status: Status::Idle,
                ..
            }
        ),
        "the swap ack must cleanly open the lease, got {final_state:?}"
    );
}

/// Drive a fresh lease to the opening buy-asset swap and hold that swap pending
/// (`ResponseMode::Delayed`). Returns the lease, the controller stand-in, and
/// the drawn principal.
fn drive_to_buy_asset_swap<DownpaymentC>(
    test_case: &mut LeaseTestCase,
    downpayment: Coin<DownpaymentC>,
) -> (Addr, Addr, Coin<LpnCurrency>)
where
    DownpaymentC: CurrencyDef,
    DownpaymentC::Group: MemberOf<PaymentGroup>,
{
    let lease = super::try_init_lease(test_case, downpayment, None);

    let quote = common::leaser::query_quote::<DownpaymentC, LeaseCurrency>(
        &test_case.app,
        test_case.address_book.leaser().clone(),
        downpayment,
        None,
    );
    let exp_borrow: Coin<LpnCurrency> = quote.borrow.try_into().unwrap();

    let remote = super::TestCase::stub_pda(1);
    let controller = test_case.address_book.remote_lease_controller().clone();
    // Pre-set the passive-vault fill (the counterparty returns only the swapped
    // inputs; coins already in the output currency are re-added on the Nolus
    // side) AND the delayed mode before the swap fires: `Delayed` snapshots the
    // ack (amount_out included) at emission time, so the fill must already be in
    // place.
    swap::set_fill(&mut test_case.app, &controller, SwapFill::InputAmount);
    stub::set_response_mode(
        &mut test_case.app,
        &controller,
        op_tag::SWAP,
        ResponseMode::Delayed,
    );

    () = common::lease::transfer_out_and_reach_swap::<DownpaymentC, LpnCurrency>(
        &mut test_case.app,
        lease.clone(),
        remote,
        (downpayment, exp_borrow),
    )
    .ignore_response()
    .unwrap_response();

    (lease, controller, exp_borrow)
}

/// Deliver the held buy-asset swap ack (whose passive-vault fill was set at
/// emission time by [`drive_to_buy_asset_swap`]).
fn deliver_open_swap(test_case: &mut LeaseTestCase, controller: &Addr) {
    () = stub::deliver_pending_callback(&mut test_case.app, controller, op_tag::SWAP)
        .ignore_response()
        .unwrap_response();
}
