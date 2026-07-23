use finance::price;
use lease::{api::ExecuteMsg, error::ContractError};
use platform::coin_legacy;
use remote_lease::callback::{RemoteErrorMessage, RemoteLeaseCallback};
use sdk::{
    cosmwasm_std::{Addr, StdResult},
    cw_multi_test::AppResponse,
    testing,
};

use crate::{
    common::{
        self, USER, ibc,
        remote_lease_controller_stub::{self as stub, ResponseMode, SwapFill, op_tag},
        swap as test_swap,
        test_case::{
            TestCase,
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
    // re-emitting the swap (again held pending by the stand-in).
    let reason = RemoteErrorMessage::new("min output not fulfilled").expect("within length cap");
    () = test_case
        .app
        .execute(
            controller.clone(),
            lease.clone(),
            &ExecuteMsg::RemoteLeaseCallback(RemoteLeaseCallback::OperationErr(reason)),
            &[],
        )
        .expect("authorised swap error must retry, not revert")
        .ignore_response()
        .unwrap_response();

    // The retry succeeds: deliver the held OK ack, then bring the proceeds in.
    let paid: LpnCoin = price::total(payment, super::price_lpn_of()).unwrap();
    let mut delivered =
        stub::deliver_pending_callback(&mut test_case.app, &controller, op_tag::SWAP);
    let transfer_amount = ibc::expect_remote_transfer(
        &mut delivered,
        TestCase::DEX_CONNECTION_ID,
        TestCase::LEASE_ICA_ID,
    );
    assert_eq!(transfer_amount, coin_legacy::to_cosmwasm_on_dex(paid));
    let _ = delivered.unwrap_response();

    let lease_ica = TestCase::stub_pda(1);
    () = test_swap::deliver_transfer_in(
        &mut test_case.app,
        lease_ica,
        lease.clone(),
        &transfer_amount,
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
