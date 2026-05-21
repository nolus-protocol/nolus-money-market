//! End-to-end coverage of `ExecuteMsg::RemoteLeaseCallback` (ibc-solray#141).
//!
//! Each test drives the lease to its first dex sub-state (`OpenIca`) where
//! the dex `Handler::authz_remote_lease_callback` is reachable, then sends
//! the callback via `app.execute` and asserts the outcome at the public
//! `ExecuteMsg` boundary.
//!
//! The `OpenIca` handler is `IcaConnector`, which overrides only
//! `authz_remote_lease_callback` and `on_open_ica`. Its `on_response` /
//! `on_error` / `on_timeout` use the dex `Handler` defaults that return
//! `UnsupportedOperation`. The tests therefore observe:
//!
//! - mismatched sender → `Unauthorized` (auth gate rejects),
//! - matched sender + each variant → `UnsupportedOperation` with the
//!   handler-specific message (auth gate passes; `classify_callback`
//!   produces the right `CallbackDispatch`; control reaches `on_dex_*`
//!   which the IcaConnector handler refuses).
//!
//! Together these prove the dispatch is wired end-to-end through the
//! public `on_remote_lease_callback` entry point.

use access_control::error::Error as AccessError;
use dex::Error as DexError;
use lease::{api::ExecuteMsg, error::ContractError};
use remote_lease::{
    callback::{RemoteErrorMessage, RemoteLeaseCallback},
    response::{CloseLeaseResponse, OperationResponse},
};
use sdk::{
    cosmwasm_std::{Addr, StdError},
    testing,
};

use crate::{common, lease::LeaseCoin};

const CALLBACK_OP_HANDLE_RESPONSE: &str = "handle transaction response";
const CALLBACK_OP_HANDLE_TIMEOUT: &str = "handle transaction timeout";
const CALLBACK_OP_HANDLE_ERROR_PREFIX: &str = "handle ICA error with details";

#[test]
fn rejects_mismatched_sender() {
    let (mut test_case, lease) = setup();
    let err = send_callback(
        &mut test_case.app,
        &lease,
        testing::user(common::USER),
        RemoteLeaseCallback::OperationTimeout,
    );

    assert_dex_error(err, |dex_err| {
        matches!(
            dex_err,
            DexError::Unauthorized(AccessError::Unauthorized {})
        )
    });
}

#[test]
fn operation_timeout_reaches_on_dex_timeout() {
    let (mut test_case, lease) = setup();
    let controller = controller_addr(&test_case);
    let err = send_callback(
        &mut test_case.app,
        &lease,
        controller,
        RemoteLeaseCallback::OperationTimeout,
    );

    assert_unsupported_operation(err, CALLBACK_OP_HANDLE_TIMEOUT);
}

#[test]
fn operation_ok_reaches_on_dex_response() {
    let (mut test_case, lease) = setup();
    let controller = controller_addr(&test_case);
    let err = send_callback(
        &mut test_case.app,
        &lease,
        controller,
        RemoteLeaseCallback::OperationOk(OperationResponse::CloseLease(CloseLeaseResponse {})),
    );

    assert_unsupported_operation(err, CALLBACK_OP_HANDLE_RESPONSE);
}

#[test]
fn operation_err_reaches_on_dex_error() {
    let (mut test_case, lease) = setup();
    let controller = controller_addr(&test_case);
    let payload = RemoteErrorMessage::new("solana side rejected").expect("within the length cap");
    let err = send_callback(
        &mut test_case.app,
        &lease,
        controller,
        RemoteLeaseCallback::OperationErr(payload),
    );

    assert_unsupported_operation_starts_with(err, CALLBACK_OP_HANDLE_ERROR_PREFIX);
}

type LeaseTestCase = super::TestCase<Addr, Addr, Addr, Addr, Addr, Addr, Addr, Addr>;

fn setup() -> (LeaseTestCase, Addr) {
    let mut test_case = super::create_test_case::<super::LeaseCurrency>();
    let lease = super::try_init_lease(&mut test_case, LeaseCoin::new(10_000), None);
    (test_case, lease)
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
    .expect_err("callback must be rejected at OpenIca state")
}

#[track_caller]
fn assert_dex_error<P>(err: StdError, predicate: P)
where
    P: FnOnce(&DexError) -> bool,
{
    let contract_err = err
        .downcast_ref::<ContractError>()
        .expect("must surface as lease ContractError");
    match contract_err {
        ContractError::DexError(inner) if predicate(inner) => {}
        other => panic!("expected DexError matching predicate, got {other:?}"),
    }
}

#[track_caller]
fn assert_unsupported_operation(err: StdError, expected_op: &str) {
    let op = expect_unsupported_op(err);
    assert_eq!(expected_op, op);
}

#[track_caller]
fn assert_unsupported_operation_starts_with(err: StdError, prefix: &str) {
    let op = expect_unsupported_op(err);
    assert!(
        op.starts_with(prefix),
        "expected op to start with {prefix:?}, got {op:?}"
    );
}

#[track_caller]
fn expect_unsupported_op(err: StdError) -> String {
    let contract_err = err
        .downcast_ref::<ContractError>()
        .expect("must surface as lease ContractError");
    match contract_err {
        ContractError::DexError(DexError::UnsupportedOperation(op, _)) => op.clone(),
        other => panic!("expected DexError::UnsupportedOperation, got {other:?}"),
    }
}
