use dex::MaxSlippage;
use finance::percent::Percent;
use lease::api::limits::MaxSlippages;
use leaser::{
    ContractError,
    msg::{NewConfig, QueryMsg},
};
use sdk::{cosmwasm_std::Addr, cw_multi_test::AppResponse, testing};

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
        Some(&ContractError::Unauthorized(_))
    ));
}

#[test]
fn privileged() {
    let mut test_case = leaser_common::test_case();

    let admin = testing::user(LEASE_ADMIN);
    let leaser = test_case.address_book.leaser().clone();

    let mut new_config = Instantiator::new_config();
    new_config.lease_max_slippages.liquidation =
        MaxSlippage::unchecked(Percent::from_permille(128));

    let expected_slippages = new_config.lease_max_slippages;

    assert!(config_leases(&mut test_case.app, leaser.clone(), admin, new_config).is_ok());
    assert_eq!(expected_slippages, max_slippages(&test_case.app, leaser));
}

fn config_leases(
    app: &mut App,
    leaser: Addr,
    caller: Addr,
    new_config: NewConfig,
) -> anyhow::Result<ResponseWithInterChainMsgs<'_, AppResponse>> {
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
