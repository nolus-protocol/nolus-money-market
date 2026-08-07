use dex::MaxSlippage;
use finance::percent::Percent100;
use lease::api::limits::MaxSlippages;
use leaser::{
    ContractError,
    msg::{LeaseConfig, LeaseConfigExternal, QueryMsg},
};
use sdk::{
    cosmwasm_std::{Addr, StdResult},
    cw_multi_test::AppResponse,
    testing,
};

use crate::common::{
    LEASE_ADMIN, USER,
    leaser::{self as leaser_common, Instantiator},
    test_case::{app::App, response::ResponseWithInterChainMsgs},
};

#[test]
fn not_privileged() {
    let mut test_case = leaser_common::test_case();

    let user = testing::user(USER);
    let leaser = test_case.address_book.leaser().clone();

    assert!(matches!(
        config_leases(&mut test_case.app, leaser, user, Instantiator::new_config())
            .expect_err("config by non authorized user should fail")
            .downcast_ref::<ContractError>(),
        Some(&ContractError::CheckPermission(_))
    ));
}

#[test]
fn privileged() {
    let mut test_case = leaser_common::test_case();

    let admin = testing::user(LEASE_ADMIN);
    let leaser = test_case.address_book.leaser().clone();

    let expected_slippages = MaxSlippages {
        open: MaxSlippage::unchecked(Percent100::from_permille(125)),
        repay: MaxSlippage::unchecked(Percent100::from_permille(126)),
        close: MaxSlippage::unchecked(Percent100::from_permille(127)),
        liquidation: MaxSlippage::unchecked(Percent100::from_permille(128)),
    };
    let new_config = LeaseConfigExternal::try_from(LeaseConfig {
        interest_rate_margin: Instantiator::INTEREST_RATE_MARGIN,
        position_spec: Instantiator::position_spec(),
        due_period: Instantiator::REPAYMENT_PERIOD,
        max_slippages: expected_slippages,
    })
    .unwrap();

    assert!(config_leases(&mut test_case.app, leaser.clone(), admin, new_config).is_ok());
    assert_eq!(expected_slippages, max_slippages(&test_case.app, leaser));
}

fn config_leases(
    app: &mut App,
    leaser: Addr,
    caller: Addr,
    new_config: LeaseConfigExternal,
) -> StdResult<ResponseWithInterChainMsgs<'_, AppResponse>> {
    app.execute(
        caller,
        leaser,
        &leaser::msg::ExecuteMsg::ConfigLeases(new_config),
        &[],
    )
}

fn max_slippages(app: &App, leaser: Addr) -> MaxSlippages {
    app.query()
        .query_wasm_smart(leaser, &QueryMsg::MaxSlippages {})
        .unwrap()
}
