//! End-to-end coverage of `ExecuteMsg::RemoteLeaseCallback` on the opening
//! buy-asset swap.
//!
//! Drives the lease to `BuyAsset` (post-transfers, swap-pending — a real
//! `SwapExactIn` dex sub-state) with the controller stand-in holding the swap
//! ack (`ResponseMode::Delayed`), then exercises the public entry point with:
//!
//! - mismatched sender → `DexError::Unauthorized` (the auth gate rejects any
//!   sender other than the registered remote-lease controller),
//! - matched sender + `OperationTimeout` → `on_dex_timeout` runs and retries
//!   the swap; the surface call commits and the lease stays in the buy-asset
//!   sub-state,
//! - matched sender + `OperationErr` → `on_dex_error` runs and retries; same,
//! - matched sender + a real `OperationResponse::Swap` OK → the in-lease JSON
//!   decoder accepts the ack and the buy-asset swap finishes, reaching
//!   `Opened`. (Pre-#142 this ack was decoded as protobuf and the success path
//!   could only be stubbed; the decoder is JSON now, so the real transition is
//!   asserted.)

use access_control::error::Error as AccessError;
use currencies::PaymentGroup;
use dex::Error as DexError;
use finance::{coin::Coin, fraction::Unit};
use lease::{
    api::{
        ExecuteMsg,
        query::{StateResponse, opened::Status},
    },
    error::ContractError,
};
use remote_lease::callback::{RemoteErrorMessage, RemoteLeaseCallback};
use sdk::{
    cosmwasm_std::{Addr, StdError},
    testing,
};

use crate::{
    common::{
        self,
        remote_lease_controller_stub::{self as stub, ResponseMode, SwapFill, op_tag},
        test_case::app::App,
    },
    lease::{LeaseCoin, LeaseCurrency, LpnCurrency},
};

type LeaseTestCase = super::TestCase<Addr, Addr, Addr, Addr, Addr, Addr, Addr, Addr>;

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
fn operation_timeout_reaches_on_dex_timeout() {
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
        .expect("authorised OperationTimeout must reach on_dex_timeout and return Ok");
    // The retry re-emits the swap (a `WasmMsg`, held pending by the stand-in),
    // so the interchain queue stays empty and the lease remains opening.
    () = response.ignore_response().unwrap_response();
    assert!(
        matches!(
            super::state_query(&test_case, lease),
            StateResponse::Opening { .. }
        ),
        "timeout must retry, keeping the lease in the opening buy-asset state",
    );
}

#[test]
fn operation_err_reaches_on_dex_error() {
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
        .expect("authorised OperationErr must reach on_dex_error and return Ok");
    // The retry re-emits the swap (held pending), same as the timeout path.
    () = response.ignore_response().unwrap_response();
    assert!(
        matches!(
            super::state_query(&test_case, lease),
            StateResponse::Opening { .. }
        ),
        "error must retry, keeping the lease in the opening buy-asset state",
    );
}

#[test]
fn operation_ok_finishes_buy_asset() {
    let (mut test_case, lease) = drive_to_swap_pending();
    let controller = controller_addr(&test_case);

    // Deliver the held, real `OperationResponse::Swap` OK from the controller.
    // The in-lease JSON decoder accepts it and the buy-asset swap finishes.
    let response = stub::deliver_pending_callback(&mut test_case.app, &controller, op_tag::SWAP);
    () = response.ignore_response().unwrap_response();

    let final_state = super::state_query(&test_case, lease);
    assert!(
        matches!(
            final_state,
            StateResponse::Opened {
                status: Status::Idle,
                ..
            }
        ),
        "a valid swap ack must finish the buy-asset swap and open a healthy lease, got {final_state:?}",
    );
}

fn drive_to_swap_pending() -> (LeaseTestCase, Addr) {
    let mut test_case = super::create_test_case::<LeaseCurrency>();
    let downpayment = LeaseCoin::new(10_000);
    let lease = super::try_init_lease(&mut test_case, downpayment, None);

    let quote = common::leaser::query_quote::<LeaseCurrency, LeaseCurrency>(
        &test_case.app,
        test_case.address_book.leaser().clone(),
        downpayment,
        None,
    );
    let exp_borrow: Coin<LpnCurrency> = quote.borrow.try_into().unwrap();

    // Single lease per test case, so the stand-in mints the first PDA.
    let remote = super::TestCase::stub_pda(1);

    let controller = test_case.address_book.remote_lease_controller().clone();
    // Hold the buy-asset swap pending so the callback entry point can be driven
    // by hand. The passive-vault fill pays only the swapped inputs (the
    // same-currency downpayment is re-added on the Nolus side), so the eventual
    // OK opens a healthy lease rather than a 100%-LTV one that would transition
    // straight into liquidation.
    stub::set_response_mode(
        &mut test_case.app,
        &controller,
        op_tag::SWAP,
        ResponseMode::Delayed,
    );
    stub::set_swap_fill(&mut test_case.app, &controller, SwapFill::InputAmount);

    () = common::lease::transfer_out_and_reach_swap::<LeaseCurrency, LpnCurrency>(
        &mut test_case.app,
        lease.clone(),
        remote,
        (downpayment, exp_borrow),
    )
    .ignore_response()
    .unwrap_response();

    (test_case, lease)
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
    callback: RemoteLeaseCallback<PaymentGroup>,
) -> StdError {
    app.execute(
        sender,
        lease.clone(),
        &ExecuteMsg::RemoteLeaseCallback(callback),
        &[],
    )
    .expect_err("callback must be rejected")
}
