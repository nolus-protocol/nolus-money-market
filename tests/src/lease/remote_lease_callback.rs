//! End-to-end coverage of `ExecuteMsg::RemoteLeaseCallback` (ibc-solray#141).
//!
//! Drives the lease to `BuyAsset` (post-transfers, swap-pending — a real
//! `SwapExactIn` dex sub-state) and exercises the public entry point with:
//!
//! - mismatched sender → `DexError::Unauthorized` (auth gate rejects),
//! - matched sender + `OperationTimeout` → real `on_dex_timeout` runs and
//!   schedules a retry — the call succeeds at the contract surface,
//! - matched sender + `OperationErr` → real `on_dex_error` runs — same,
//! - matched sender + `OperationOk` → the outer call returns `Ok` and the
//!   safe-delivery machinery activates: `on_dex_response` wraps the
//!   response in `ResponseDelivery` and emits the
//!   `SubMsg::reply_on_error(DexCallback)`; the inner `DexCallback`
//!   then runs synchronously, fails to decode the JSON payload as the
//!   chain's protobuf swap response, and the reply handler schedules
//!   a retry via `TimeAlarms`. The test asserts the surface contract
//!   (outer tx commits, `wasm-next-delivery` event is emitted) — the
//!   real success-path semantics land with ibc-solray#142, which
//!   switches the in-lease decoder to JSON.

use access_control::error::Error as AccessError;
use currencies::PaymentGroup;
use dex::Error as DexError;
use lease::{api::ExecuteMsg, error::ContractError};
use remote_lease::{
    callback::{RemoteErrorMessage, RemoteLeaseCallback},
    response::{CloseLeaseResponse, OperationResponse},
};
use sdk::{
    cosmwasm_std::{Addr, StdError},
    cw_multi_test::AppResponse,
    testing,
};
use swap::testing::SwapRequest;

use crate::{
    common::{
        self, swap as test_swap,
        test_case::{app::App, response::ResponseWithInterChainMsgs},
    },
    lease::{LeaseCoin, LeaseCurrency, LpnCoin, LpnCurrency},
};

type LeaseTestCase = super::TestCase<Addr, Addr, Addr, Addr, Addr, Addr, Addr, Addr>;

#[test]
fn rejects_mismatched_sender_at_swap_state() {
    let (mut test_case, lease, _requests) = drive_to_swap_pending();
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
fn operation_timeout_reaches_on_dex_timeout() {
    let (mut test_case, lease, _requests) = drive_to_swap_pending();
    let controller = controller_addr(&test_case);

    let response = test_case
        .app
        .execute(
            controller,
            lease,
            &ExecuteMsg::RemoteLeaseCallback(RemoteLeaseCallback::OperationTimeout),
            &[],
        )
        .expect("authorised OperationTimeout must reach on_dex_timeout and return Ok");
    // The real on_dex_timeout for SwapExactIn schedules a retry — drain the
    // resulting SubmitTx so the test_case is left with an empty queue.
    expect_swap_retry(response);
}

#[test]
fn operation_err_reaches_on_dex_error() {
    let (mut test_case, lease, _requests) = drive_to_swap_pending();
    let controller = controller_addr(&test_case);
    let payload = RemoteErrorMessage::new("solana side rejected").expect("within the length cap");

    let response = test_case
        .app
        .execute(
            controller,
            lease,
            &ExecuteMsg::RemoteLeaseCallback(RemoteLeaseCallback::OperationErr(payload)),
            &[],
        )
        .expect("authorised OperationErr must reach on_dex_error and return Ok");
    // The real on_dex_error for SwapExactIn schedules a retry — drain the
    // resulting SubmitTx so the test_case is left with an empty queue.
    expect_swap_retry(response);
}

#[test]
fn operation_ok_activates_safe_delivery() {
    let (mut test_case, lease, _requests) = drive_to_swap_pending();
    let controller = controller_addr(&test_case);
    // `CloseLeaseResponse` is a stand-in payload — see the module
    // doc-comment for why the success path is deferred to ibc-solray#142.
    let payload = OperationResponse::CloseLease(CloseLeaseResponse {});

    let response = test_case
        .app
        .execute(
            controller,
            lease,
            &ExecuteMsg::RemoteLeaseCallback(RemoteLeaseCallback::OperationOk(payload)),
            &[],
        )
        .expect("authorised OperationOk: outer tx must commit via the safe-delivery wrapper");
    let app_response = response.unwrap_response();

    // Safe-delivery proof: the outer `on_dex_response` returned Ok after
    // wrapping in `ResponseDelivery` + scheduling `DexCallback`; the inner
    // `DexCallback` failed the protobuf decode; the reply handler caught
    // the failure and scheduled the retry via `TimeAlarms` — emitting the
    // `wasm-next-delivery` event with `what = dex-response`.
    let event_observed = app_response.events.iter().any(|event| {
        event.ty == "wasm-next-delivery"
            && event
                .attributes
                .iter()
                .any(|attr| attr.key == "what" && attr.value == "dex-response")
    });
    assert!(
        event_observed,
        "expected the safe-delivery retry to be scheduled (`wasm-next-delivery`), got events {:?}",
        app_response.events,
    );
}

fn drive_to_swap_pending() -> (LeaseTestCase, Addr, Vec<SwapRequest<PaymentGroup>>) {
    let mut test_case = super::create_test_case::<LeaseCurrency>();
    let downpayment = LeaseCoin::new(10_000);
    let lease = super::try_init_lease(&mut test_case, downpayment, None);

    let quote = common::leaser::query_quote::<LeaseCurrency, LeaseCurrency>(
        &test_case.app,
        test_case.address_book.leaser().clone(),
        downpayment,
        None,
    );
    let exp_borrow: LpnCoin = quote.borrow.try_into().unwrap();

    // Single lease per test case, so the stand-in mints the first PDA.
    let remote = super::TestCase::stub_pda(1);

    let response = common::lease::transfer_out_and_reach_swap::<LeaseCurrency, LpnCurrency>(
        &mut test_case.app,
        lease.clone(),
        remote,
        (downpayment, exp_borrow),
    );

    let requests = test_swap::expect_swap(
        response,
        super::TestCase::DEX_CONNECTION_ID,
        super::TestCase::LEASE_ICA_ID,
        |_response| {},
    );

    (test_case, lease, requests)
}

fn controller_addr(test_case: &LeaseTestCase) -> Addr {
    // The controller stand-in is registered alongside the leaser in
    // `init_leaser` and exposed on the address book — see
    // `common::remote_lease_controller_stub`.
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

fn expect_swap_retry(response: ResponseWithInterChainMsgs<'_, AppResponse>) {
    let _retry = test_swap::expect_swap(
        response,
        super::TestCase::DEX_CONNECTION_ID,
        super::TestCase::LEASE_ICA_ID,
        |_response| {},
    );
}
