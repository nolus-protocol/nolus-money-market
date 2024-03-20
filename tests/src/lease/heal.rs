use lease::{api::ExecuteMsg, error::ContractError};
use sdk::{cosmwasm_std::Addr, cw_multi_test::AppResponse};

use crate::{
    common::{
        cwcoin,
        test_case::{
            response::{RemoteChain, ResponseWithInterChainMsgs},
            TestCase,
        },
        USER,
    },
    lease::{LeaseCoin, LeaseCurrency, LpnCoin, LpnCurrency},
};

#[test]
fn active_state() {
    let mut test_case = super::create_test_case::<LeaseCurrency>();
    let downpayment = LeaseCoin::new(10_000);
    let lease = super::open_lease(&mut test_case, downpayment, None);

    let query_result = super::state_query(&test_case, &lease.clone().into_string());
    let expected_result =
        super::expected_newly_opened_state(&test_case, downpayment, super::create_payment_coin(0));
    assert_eq!(query_result, expected_result);

    let unutilized_amount: LpnCoin = 100.into();

    test_case.send_funds_from_admin(lease.clone(), &[cwcoin(unutilized_amount)]);
    heal_ok(&mut test_case, lease.clone());
    assert!(
        platform::bank::balance::<LpnCurrency>(&lease, test_case.app.query())
            .unwrap()
            .is_zero()
    );

    let query_result = super::state_query(&test_case, &lease.into_string());
    let expected_result =
        super::expected_newly_opened_state(&test_case, downpayment, unutilized_amount);
    assert_eq!(query_result, expected_result);
}

pub(super) fn heal_no_inconsistency<
    ProtocolsRegistry,
    Dispatcher,
    Treasury,
    Profit,
    Reserve,
    Leaser,
    Lpp,
    Oracle,
    TimeAlarms,
>(
    test_case: &mut TestCase<
        ProtocolsRegistry,
        Dispatcher,
        Treasury,
        Profit,
        Reserve,
        Leaser,
        Lpp,
        Oracle,
        TimeAlarms,
    >,
    lease: Addr,
) {
    let err = try_heal(test_case, lease).unwrap_err();
    let heal_err = err.downcast_ref::<ContractError>();
    assert_eq!(Some(&ContractError::InconsistencyNotDetected()), heal_err);
}

pub(super) fn heal_unsupported<
    ProtocolsRegistry,
    Dispatcher,
    Treasury,
    Profit,
    Reserve,
    Leaser,
    Lpp,
    Oracle,
    TimeAlarms,
>(
    test_case: &mut TestCase<
        ProtocolsRegistry,
        Dispatcher,
        Treasury,
        Profit,
        Reserve,
        Leaser,
        Lpp,
        Oracle,
        TimeAlarms,
    >,
    lease: Addr,
) {
    let err = try_heal(test_case, lease).unwrap_err();
    let heal_err = err.downcast_ref::<ContractError>();
    assert_eq!(
        Some(&ContractError::unsupported_operation("heal")),
        heal_err
    );
}

fn try_heal<
    ProtocolsRegistry,
    Dispatcher,
    Treasury,
    Profit,
    Reserve,
    Leaser,
    Lpp,
    Oracle,
    TimeAlarms,
>(
    test_case: &mut TestCase<
        ProtocolsRegistry,
        Dispatcher,
        Treasury,
        Profit,
        Reserve,
        Leaser,
        Lpp,
        Oracle,
        TimeAlarms,
    >,
    lease: Addr,
) -> anyhow::Result<ResponseWithInterChainMsgs<'_, AppResponse>> {
    test_case
        .app
        .execute(Addr::unchecked(USER), lease, &ExecuteMsg::Heal(), &[])
}

fn heal_ok<
    ProtocolsRegistry,
    Dispatcher,
    Treasury,
    Profit,
    Reserve,
    Leaser,
    Lpp,
    Oracle,
    TimeAlarms,
>(
    test_case: &mut TestCase<
        ProtocolsRegistry,
        Dispatcher,
        Treasury,
        Profit,
        Reserve,
        Leaser,
        Lpp,
        Oracle,
        TimeAlarms,
    >,
    lease: Addr,
) {
    let mut response = try_heal(test_case, lease).unwrap();
    response.expect_empty();

    let heal_resp = response.unwrap_response();
    assert!(heal_resp.data.is_none());
}
