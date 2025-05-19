use lease::{api::ExecuteMsg, error::ContractError};
use sdk::{cosmwasm_std::Addr, cw_multi_test::AppResponse, testing};

use crate::{
    common::{
        self, USER, cwcoin, swap as test_swap,
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

    let unutilized_amount: LpnCoin = 100.into();

    test_case.send_funds_from_admin(lease.clone(), &[cwcoin(unutilized_amount)]);
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
    test_case.send_funds_from_admin(testing::user(USER), &[cwcoin(payment)]);

    repay::repay_with_hook_on_swap(&mut test_case, lease.clone(), payment, |ref mut app| {
        let swap_response_retry = common::swap::do_swap_with_error(app, lease.clone())
            .expect("on error should have retried again the swap");

        test_swap::expect_swap(
            swap_response_retry,
            TestCase::DEX_CONNECTION_ID,
            TestCase::LEASE_ICA_ID,
            |_| {},
        );
    });

    let query_result = super::state_query(&test_case, lease.clone());
    let expected_result = super::expected_newly_opened_state(&test_case, downpayment, payment);
    assert_eq!(query_result, expected_result);

    heal_no_inconsistency(&mut test_case.app, lease, testing::user(USER));
}

pub(super) fn heal_no_inconsistency(app: &mut App, lease: Addr, caller: Addr) {
    let err = try_heal(app, lease, caller).unwrap_err();
    let heal_err = err.downcast_ref::<ContractError>();
    assert_eq!(Some(&ContractError::InconsistencyNotDetected()), heal_err);
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
) -> anyhow::Result<ResponseWithInterChainMsgs<'_, AppResponse>> {
    app.execute(caller, lease, &ExecuteMsg::Heal(), &[])
}
