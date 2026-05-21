//! End-to-end coverage of `ExecuteMsg::RemoteLeaseCallback` (ibc-solray#141).
//!
//! Drives the lease to `BuyAsset` (post-transfers, swap-pending — a real
//! `SwapExactIn` dex sub-state) and exercises the public entry point with:
//!
//! - mismatched sender → `DexError::Unauthorized` (auth gate rejects),
//! - matched sender + `OperationTimeout` → real `on_dex_timeout` runs and
//!   schedules a retry — the call succeeds at the contract surface,
//! - matched sender + `OperationErr` → real `on_dex_error` runs — same.
//!
//! `OperationOk` is intentionally not covered here. The current dex
//! `Handler::on_response` decodes `data` as a protobuf swap response from
//! the chain; the `RemoteLeaseCallback` path will not deliver responses
//! that satisfy that contract until ibc-solray#142 switches the lease
//! lifecycle calls to `remote_lease` stubs. Until then, an authorised
//! `OperationOk` produces a decoding failure rather than meaningful
//! semantics; the unit-level coverage in `state/dex.rs::classify_callback`
//! and `api/mod.rs` pins the wire shape and the dispatch.

use access_control::error::Error as AccessError;
use currencies::PaymentGroup;
use dex::Error as DexError;
use lease::{api::ExecuteMsg, error::ContractError};
use remote_lease::callback::{RemoteErrorMessage, RemoteLeaseCallback};
use sdk::{
    cosmwasm_std::{Addr, StdError},
    testing,
};
use swap::testing::SwapRequest;

use crate::{
    common::{self, swap as test_swap},
    lease::{LeaseCoin, LeaseCurrency, LpnCoin, LpnCurrency},
};

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

fn expect_swap_retry(
    response: crate::common::test_case::response::ResponseWithInterChainMsgs<
        '_,
        sdk::cw_multi_test::AppResponse,
    >,
) {
    let _retry = test_swap::expect_swap(
        response,
        super::TestCase::DEX_CONNECTION_ID,
        super::TestCase::LEASE_ICA_ID,
        |_| {},
    );
}

type LeaseTestCase = super::TestCase<Addr, Addr, Addr, Addr, Addr, Addr, Addr, Addr>;

fn drive_to_swap_pending() -> (
    LeaseTestCase,
    Addr,
    Vec<SwapRequest<PaymentGroup, PaymentGroup>>,
) {
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

    let requests = test_swap::expect_swap(
        response,
        super::TestCase::DEX_CONNECTION_ID,
        super::TestCase::LEASE_ICA_ID,
        |_| {},
    );

    (test_case, lease, requests)
}

fn controller_addr(test_case: &LeaseTestCase) -> Addr {
    // Mirrors the `Instantiator::instantiate` stand-in: the `reserve`
    // contract address is configured as each lease's `remote_lease`
    // controller in the integration-test harness.
    test_case.address_book.reserve().clone()
}

fn send_callback(
    app: &mut crate::common::test_case::app::App,
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
