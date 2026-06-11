//! End-to-end coverage of `ExecuteMsg::RemoteLeaseCallback` (ibc-solray#141).
//!
//! Drives the lease to the remote swap leg (post-transfers, swap-pending —
//! the controller stand-in defers its `Swap` acknowledgment) and exercises
//! the public entry point with:
//!
//! - mismatched sender → `DexError::Unauthorized` (auth gate rejects),
//! - matched sender + `OperationTimeout` → the in-flight leg is re-emitted
//!   (`timeout = retry` event) — the call succeeds at the contract surface,
//! - matched sender + `OperationErr` → same per-leg retry,
//! - matched sender + a non-`Swap` `OperationOk` → absorbed with an
//!   `absorbed = undecodable-response` event; the state does not advance
//!   and the controller's acknowledgment transaction commits.

use access_control::error::Error as AccessError;
use dex::Error as DexError;
use lease::{
    api::{
        ExecuteMsg,
        query::{StateResponse, opening::OngoingTrx as OpeningOngoingTrx},
    },
    error::ContractError,
};
use remote_lease::{
    callback::{RemoteErrorMessage, RemoteLeaseCallback},
    response::{CloseLeaseResponse, WireOperationResponse},
};
use sdk::{
    cosmwasm_std::{Addr, Event, StdError},
    testing,
};

use crate::{
    common::{
        self,
        remote_lease_controller_stub::{self as stub, ResponseMode, op_tag},
        test_case::app::App,
    },
    lease::{LeaseCoin, LeaseCurrency, LpnCoin, LpnCurrency},
};

type LeaseTestCase = super::TestCase<Addr, Addr, Addr, Addr, Addr, Addr, Addr, Addr>;

const OPENING_SWAP_EVENT: &str = "wasm-ls-open-swap";

#[test]
fn rejects_mismatched_sender_at_swap_state() {
    let (mut test_case, lease) = drive_to_swap_pending();
    let err = send_callback(
        &mut test_case.app,
        &lease,
        testing::user(common::USER),
        RemoteLeaseCallback::OperationTimeout,
    );

    let contract_err = err
        .downcast_ref::<ContractError>()
        .expect("must surface as lease ContractError");
    assert!(
        matches!(
            contract_err,
            ContractError::DexError(DexError::Unauthorized(AccessError::Unauthorized {}))
        ),
        "expected DexError::Unauthorized, got {contract_err:?}"
    );
}

#[test]
fn operation_timeout_retries_the_in_flight_leg() {
    let (mut test_case, lease) = drive_to_swap_pending();
    let controller = controller_addr(&test_case);

    let response = test_case
        .app
        .execute(
            controller,
            lease.clone(),
            &ExecuteMsg::RemoteLeaseCallback(RemoteLeaseCallback::OperationTimeout),
            &[],
        )
        .expect("authorised OperationTimeout must re-emit the leg and return Ok")
        .unwrap_response();
    expect_attribute(&response.events, OPENING_SWAP_EVENT, "timeout", "retry");
    assert_swap_pending(&test_case, lease);
}

#[test]
fn operation_err_retries_the_in_flight_leg() {
    let (mut test_case, lease) = drive_to_swap_pending();
    let controller = controller_addr(&test_case);
    let payload = RemoteErrorMessage::new("solana side rejected").expect("within the length cap");

    let response = test_case
        .app
        .execute(
            controller,
            lease.clone(),
            &ExecuteMsg::RemoteLeaseCallback(RemoteLeaseCallback::OperationErr(payload)),
            &[],
        )
        .expect("authorised OperationErr must re-emit the leg and return Ok")
        .unwrap_response();
    expect_attribute(&response.events, OPENING_SWAP_EVENT, "timeout", "retry");
    assert_swap_pending(&test_case, lease);
}

#[test]
fn non_swap_operation_ok_is_absorbed() {
    let (mut test_case, lease) = drive_to_swap_pending();
    let controller = controller_addr(&test_case);
    let payload = WireOperationResponse::CloseLease(CloseLeaseResponse {});

    let response = test_case
        .app
        .execute(
            controller,
            lease.clone(),
            &ExecuteMsg::RemoteLeaseCallback(RemoteLeaseCallback::OperationOk(payload)),
            &[],
        )
        .expect("a non-swap success ack must be absorbed, committing the controller's tx")
        .unwrap_response();
    expect_attribute(
        &response.events,
        OPENING_SWAP_EVENT,
        "absorbed",
        "unexpected-response-variant",
    );
    assert_swap_pending(&test_case, lease);
}

fn drive_to_swap_pending() -> (LeaseTestCase, Addr) {
    let mut test_case = super::create_test_case::<LeaseCurrency>();
    let downpayment = LeaseCoin::new(10_000);
    let lease = super::try_init_lease(&mut test_case, downpayment, None);

    let controller = controller_addr(&test_case);
    stub::set_response_mode(
        &mut test_case.app,
        &controller,
        op_tag::SWAP,
        ResponseMode::Delayed,
    );

    let quote = common::leaser::query_quote::<LeaseCurrency, LeaseCurrency>(
        &test_case.app,
        test_case.address_book.leaser().clone(),
        downpayment,
        None,
    );
    let exp_borrow: LpnCoin = quote.borrow.try_into().unwrap();

    let ica_addr = super::TestCase::ica_addr(&lease, super::TestCase::LEASE_ICA_ID);
    let ica_port = format!("icacontroller-{ica_addr}");
    let ica_channel = format!("channel-{ica_addr}");

    let response = common::lease::confirm_ica_and_transfer_funds::<LeaseCurrency, LpnCurrency>(
        &mut test_case.app,
        lease.clone(),
        super::TestCase::DEX_CONNECTION_ID,
        (&ica_channel, &ica_port, ica_addr),
        (downpayment, exp_borrow),
    );
    () = response.ignore_response().unwrap_response();

    assert_swap_pending(&test_case, lease.clone());

    (test_case, lease)
}

fn assert_swap_pending(test_case: &LeaseTestCase, lease: Addr) {
    // The downpayment is already in the lease currency, so the only swap
    // leg is the LPN borrow.
    match super::state_query(test_case, lease) {
        StateResponse::Opening {
            in_progress: OpeningOngoingTrx::BuyAsset { acks_left },
            ..
        } => assert_eq!(1, acks_left),
        other => panic!("expected the in-flight swap leg, got {other:?}"),
    }
}

fn controller_addr(test_case: &LeaseTestCase) -> Addr {
    test_case.address_book.remote_lease_controller().clone()
}

fn send_callback(
    app: &mut App,
    lease: &Addr,
    sender: Addr,
    callback: RemoteLeaseCallback,
) -> StdError {
    app.execute(
        sender,
        lease.clone(),
        &ExecuteMsg::RemoteLeaseCallback(callback),
        &[],
    )
    .expect_err("callback must be rejected")
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
