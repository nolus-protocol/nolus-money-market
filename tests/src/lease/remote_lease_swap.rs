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

use currencies::PaymentGroup;
use currency::{CurrencyDef, MemberOf};
use finance::{
    coin::{Coin, CoinDTO},
    duration::Duration,
};
use lease::api::{
    ExecuteMsg,
    query::{StateResponse, opening::OngoingTrx as OpeningOngoingTrx},
};
use remote_lease::{
    callback::RemoteLeaseCallback,
    response::{OperationResponse, SwapResponse, TransferOutResponse},
};
use sdk::{
    cosmwasm_std::{Addr, Event},
    cw_multi_test::AppResponse,
    testing,
};

use crate::common::{
    self, ADMIN,
    remote_lease_controller_stub::{self as stub, ResponseMode, op_tag},
    test_case::TestCase,
};

use super::{DOWNPAYMENT, LeaseCoin, LeaseCurrency, LeaseTestCase, LpnCoin, LpnCurrency};

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

    let ica_addr = TestCase::ica_addr(&lease, TestCase::LEASE_ICA_ID);
    let ica_port = format!("icacontroller-{ica_addr}");
    let ica_channel = format!("channel-{ica_addr}");

    let mut response = common::lease::send_open_ica_response(
        &mut test_case.app,
        lease.clone(),
        TestCase::DEX_CONNECTION_ID,
        &ica_channel,
        &ica_port,
        ica_addr.as_str(),
    )
    .ignore_response();
    let downpayment_cw = common::ibc::expect_transfer(
        &mut response,
        TestCase::LEASER_IBC_CHANNEL,
        lease.as_str(),
        ica_addr.as_str(),
    );
    let borrow_cw = common::ibc::expect_transfer(
        &mut response,
        TestCase::LEASER_IBC_CHANNEL,
        lease.as_str(),
        ica_addr.as_str(),
    );
    () = response.unwrap_response();
    assert_transfers_pending(&test_case, lease.clone());

    // a swap acknowledgment arrives while both transfer acknowledgments
    // are still outstanding - it must neither error nor advance the
    // transfer countdown
    let unexpected = RemoteLeaseCallback::OperationOk(OperationResponse::Swap(SwapResponse {
        amount_out: Coin::<LeaseCurrency>::new(1).into(),
    }));
    let injected = stub::inject_callback(&mut test_case.app, &controller, &lease, unexpected);
    expect_attribute(&injected.events, ABSORB_EVENT, "absorbed", "response");
    assert_transfers_pending(&test_case, lease.clone());

    () = common::ibc::do_transfer(
        &mut test_case.app,
        lease.clone(),
        ica_addr.clone(),
        false,
        &downpayment_cw,
    )
    .ignore_response()
    .unwrap_response();
    () = common::ibc::do_transfer(
        &mut test_case.app,
        lease.clone(),
        ica_addr,
        false,
        &borrow_cw,
    )
    .ignore_response()
    .unwrap_response();

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

    let wrong_variant =
        RemoteLeaseCallback::OperationOk(OperationResponse::TransferOut(TransferOutResponse {}));
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
    let ica_port = format!("icacontroller-{ica_addr}");
    let ica_channel = format!("channel-{ica_addr}");

    common::lease::confirm_ica_and_transfer_funds::<DownpaymentC, LpnCurrency>(
        &mut test_case.app,
        lease.clone(),
        TestCase::DEX_CONNECTION_ID,
        (&ica_channel, &ica_port, ica_addr),
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
            in_progress: OpeningOngoingTrx::TransferOut { .. },
            ..
        } => (),
        other => panic!("expected the transfer-out leg, got {other:?}"),
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
