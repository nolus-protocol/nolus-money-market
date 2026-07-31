use currencies::PaymentGroup;
use finance::{coin::CoinDTO, price};
use lease::{api::ExecuteMsg, error::ContractError};
use remote_lease::callback::{RemoteErrorKind, RemoteLeaseCallback};
use sdk::{
    cosmwasm_std::{Addr, StdResult},
    cw_multi_test::AppResponse,
    testing,
};

use crate::{
    common::{
        self, USER,
        remote_lease_controller_stub::{self as stub, ResponseMode, SwapFill, op_tag},
        swap as test_swap,
        test_case::{
            app::App,
            response::{RemoteChain, ResponseWithInterChainMsgs},
        },
    },
    lease::{LeaseCoin, LeaseCurrency, LpnCoin, LpnCurrency, repay},
};

#[test]
fn active_state() {
    let mut test_case = super::create_test_case::<LeaseCurrency>();
    let downpayment = LeaseCoin::new(10_000);
    let lease = super::open_lease(&mut test_case, downpayment, None);

    let query_result = super::state_query(&test_case, lease.clone());
    let expected_result =
        super::expected_newly_opened_state(&test_case, downpayment, super::create_payment_coin(0));
    assert_eq!(query_result, expected_result);

    let unutilized_amount: LpnCoin = common::coin(100);

    test_case.send_funds_from_admin(lease.clone(), &[common::cwcoin(unutilized_amount)]);
    heal_ok(&mut test_case.app, lease.clone(), testing::user(USER))
        .ignore_response()
        .expect_empty();
    assert!(
        platform::bank::balance::<LpnCurrency>(&lease, test_case.app.query())
            .unwrap()
            .is_zero()
    );

    let query_result = super::state_query(&test_case, lease.clone());
    let expected_result =
        super::expected_newly_opened_state(&test_case, downpayment, unutilized_amount);
    assert_eq!(query_result, expected_result);

    heal_no_inconsistency(&mut test_case.app, lease, testing::user(USER));
}

// Pins decision D7 on the repay buy-LPN leg. Its calculator accepts any
// non-zero swap, so there is no floor for an output to fall below and a
// `MinOutUnmet` ack must be treated exactly like any other cause — retry, never
// park. The sibling `swap_on_repay` injects `Permanent`, which maps to
// `AnomalyCause::Other`, so it stays green even if the below-floor cause were
// wrongly made to park on every leg; only this injection discriminates.
#[test]
fn min_out_unmet_still_retries_on_repay() {
    let mut test_case = super::create_test_case::<LeaseCurrency>();
    let downpayment = LeaseCoin::new(10_000);
    let lease = super::open_lease(&mut test_case, downpayment, None);

    let payment = super::create_payment_coin(1_000);
    test_case.send_funds_from_admin(testing::user(USER), &[common::cwcoin(payment)]);

    let controller = test_case.address_book.remote_lease_controller().clone();
    stub::set_response_mode(
        &mut test_case.app,
        &controller,
        op_tag::SWAP,
        ResponseMode::Delayed,
    );
    stub::set_swap_fill(&mut test_case.app, &controller, SwapFill::InputAmount);

    () = repay::send_payment_and_transfer(&mut test_case, lease.clone(), payment)
        .ignore_response()
        .unwrap_response();

    let swaps_before = test_swap::count(&test_case.app, &controller);

    let app_response = test_case
        .app
        .execute(
            controller.clone(),
            lease,
            &ExecuteMsg::RemoteLeaseCallback(RemoteLeaseCallback::OperationErr(stub::error_ack(
                RemoteErrorKind::MinOutUnmet,
                "ibc-solray: post-swap credit below required min",
            ))),
            &[],
        )
        .expect("a below-floor ack on a floorless leg must retry, not revert")
        .unwrap_response();

    assert_eq!(
        swaps_before + 1,
        test_swap::count(&test_case.app, &controller),
        "the buy-LPN swap must be re-emitted",
    );
    assert!(
        !app_response
            .events
            .iter()
            .any(|event| event.ty == "wasm-ls-slippage-anomaly"),
        "the repay leg must never claim slippage protection",
    );
}

#[test]
fn swap_on_repay() {
    let mut test_case = super::create_test_case::<LeaseCurrency>();
    let downpayment = LeaseCoin::new(10_000);
    let lease = super::open_lease(&mut test_case, downpayment, None);

    let query_result = super::state_query(&test_case, lease.clone());
    let expected_result =
        super::expected_newly_opened_state(&test_case, downpayment, super::create_payment_coin(0));
    assert_eq!(query_result, expected_result);

    let payment = super::create_payment_coin(1_000);
    test_case.send_funds_from_admin(testing::user(USER), &[common::cwcoin(payment)]);

    let controller = test_case.address_book.remote_lease_controller().clone();
    // Hold the buy-LPN swap pending so the failure-then-retry can be driven by
    // hand; the eventual identity fill yields the payment's LPN value.
    stub::set_response_mode(
        &mut test_case.app,
        &controller,
        op_tag::SWAP,
        ResponseMode::Delayed,
    );
    stub::set_swap_fill(&mut test_case.app, &controller, SwapFill::InputAmount);

    // The payment is transferred out and the buy-LPN swap emitted, then held.
    () = repay::send_payment_and_transfer(&mut test_case, lease.clone(), payment)
        .ignore_response()
        .unwrap_response();

    // The counterparty rejects the first swap; the buy-LPN task retries,
    // re-emitting the swap (again held pending by the stand-in). The retry comes
    // from the leg's anomaly treatment, not from the cause; the `Permanent`
    // framing keeps the injection honest for when the lease does route on it.
    () = test_case
        .app
        .execute(
            controller.clone(),
            lease.clone(),
            &ExecuteMsg::RemoteLeaseCallback(RemoteLeaseCallback::OperationErr(stub::error_ack(
                RemoteErrorKind::Permanent,
                "jupiter route decode failed",
            ))),
            &[],
        )
        .expect("authorised swap error must retry, not revert")
        .ignore_response()
        .unwrap_response();

    // The retry succeeds: deliver the held OK ack — the swap and the proceeds
    // transfer-out fire inline, parking the lease in TransferInFinish — then
    // bring the proceeds in.
    let paid: LpnCoin = price::total(payment, super::price_lpn_of()).unwrap();
    let _ = stub::deliver_pending_callback(&mut test_case.app, &controller, op_tag::SWAP)
        .unwrap_response();

    // Fidelity: the emitted transfer-out returns exactly the LPN proceeds.
    assert_eq!(
        Into::<CoinDTO<PaymentGroup>>::into(paid),
        test_swap::captured_transfer_out(&test_case.app, &controller),
    );

    let time_alarms = test_case.address_book.time_alarms().clone();
    () = test_swap::deliver_transfer_in(
        &mut test_case.app,
        time_alarms,
        lease.clone(),
        &common::cwcoin(paid),
    )
    .ignore_response()
    .unwrap_response();

    let query_result = super::state_query(&test_case, lease.clone());
    let expected_result = super::expected_newly_opened_state(&test_case, downpayment, payment);
    assert_eq!(query_result, expected_result);

    heal_no_inconsistency(&mut test_case.app, lease, testing::user(USER));
}

pub(super) fn heal_no_inconsistency(app: &mut App, lease: Addr, caller: Addr) {
    let err = try_heal(app, lease, caller).unwrap_err();
    assert!(matches!(
        err.downcast_ref::<ContractError>().unwrap(),
        &ContractError::InconsistencyNotDetected()
    ));
}

pub(super) fn heal_no_rights(app: &mut App, lease: Addr, caller: Addr) {
    let err = try_heal(app, lease, caller).unwrap_err();
    assert!(matches!(
        err.downcast_ref::<ContractError>().unwrap(),
        &ContractError::Unauthorized(access_control::error::Error::Unauthorized {})
    ));
}

// pub(super) fn heal_unsupported(app: &mut App, lease: Addr) {
//     let err = try_heal(app, lease).unwrap_err();
//     let heal_err = err.downcast_ref::<ContractError>();
//     assert_eq!(
//         Some(&ContractError::unsupported_operation("heal")),
//         heal_err
//     );
// }

pub(super) fn heal_ok(
    app: &mut App,
    lease: Addr,
    caller: Addr,
) -> ResponseWithInterChainMsgs<'_, AppResponse> {
    try_heal(app, lease, caller).unwrap()
}

fn try_heal(
    app: &mut App,
    lease: Addr,
    caller: Addr,
) -> StdResult<ResponseWithInterChainMsgs<'_, AppResponse>> {
    app.execute(caller, lease, &ExecuteMsg::Heal(), &[])
}
