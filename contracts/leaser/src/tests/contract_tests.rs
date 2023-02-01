use serde::{Deserialize, Serialize};

use access_control::Unauthorized;
use finance::{
    currency::Currency, duration::Duration, liability::Liability, percent::Percent,
    test::currency::Usdc,
};
use lease::api::{
    dex::{ConnectionParams, Ics20Channel},
    InterestPaymentSpec,
};
use sdk::{
    cosmwasm_ext::Response,
    cosmwasm_std::{
        coins, from_binary,
        testing::{mock_env, mock_info},
        to_binary, Addr, CosmosMsg, Deps, DepsMut, MessageInfo, SubMsg, Uint64, WasmMsg,
    },
    schemars::{self, JsonSchema},
    testing::mock_deps_with_contracts,
};

use crate::{
    cmd::Borrow,
    contract::{execute, instantiate, query},
    error::ContractResult,
    msg::{ConfigResponse, ExecuteMsg, QueryMsg},
    state::config::Config,
    ContractError,
};

const CREATOR: &str = "creator";
const LPP_ADDR: &str = "test";
const TIMEALARMS_ADDR: &str = "timealarms";
const ORACLE_ADDR: &str = "oracle";
const PROFIT_ADDR: &str = "profit";

type TheCurrency = Usdc;

const DENOM: &str = TheCurrency::TICKER;
const MARGIN_INTEREST_RATE: Percent = Percent::from_permille(30);

fn leaser_instantiate_msg(lease_code_id: u64, lpp_addr: Addr) -> crate::msg::InstantiateMsg {
    crate::msg::InstantiateMsg {
        lease_code_id: Uint64::new(lease_code_id),
        lpp_ust_addr: lpp_addr,
        lease_interest_rate_margin: MARGIN_INTEREST_RATE,
        liability: Liability::new(
            Percent::from_percent(65),
            Percent::from_percent(5),
            Percent::from_percent(10),
            Percent::from_percent(2),
            Percent::from_percent(3),
            Percent::from_percent(2),
            Duration::from_hours(1),
        ),
        lease_interest_payment: InterestPaymentSpec::new(
            Duration::from_days(90),
            Duration::from_days(10),
        ),
        time_alarms: Addr::unchecked(TIMEALARMS_ADDR),
        market_price_oracle: Addr::unchecked(ORACLE_ADDR),
        profit: Addr::unchecked(PROFIT_ADDR),
    }
}

fn owner() -> MessageInfo {
    mock_info(CREATOR, &coins(2, DENOM))
}

fn customer() -> MessageInfo {
    mock_info("addr0000", &coins(2, DENOM))
}

fn setup_test_case(deps: DepsMut) {
    let lpp_addr = Addr::unchecked(LPP_ADDR);
    let msg = leaser_instantiate_msg(1, lpp_addr);

    let resp = instantiate(deps, mock_env(), owner(), msg).unwrap();
    assert_eq!(1, resp.messages.len());
}

fn query_config(deps: Deps) -> Config {
    let res = query(deps, mock_env(), QueryMsg::Config {}).unwrap();
    let config_response: ConfigResponse = from_binary(&res).unwrap();
    config_response.config
}

fn dex_params() -> ConnectionParams {
    ConnectionParams {
        connection_id: "connection-0".into(),
        transfer_channel: Ics20Channel {
            local_endpoint: "channel-0".into(),
            remote_endpoint: "channel-2048".into(),
        },
    }
}

fn setup_dex_ok(deps: DepsMut) {
    let resp = setup_dex(deps, owner()).expect("dex update passed");
    assert!(resp.messages.is_empty());
}

fn setup_dex(deps: DepsMut, info: MessageInfo) -> ContractResult<Response> {
    let msg = ExecuteMsg::SetupDex(dex_params());
    execute(deps, mock_env(), info, msg)
}

#[test]
fn proper_initialization() {
    let mut deps = mock_deps_with_contracts([LPP_ADDR, TIMEALARMS_ADDR, PROFIT_ADDR, ORACLE_ADDR]);

    let lpp_addr = Addr::unchecked(LPP_ADDR);
    let msg = leaser_instantiate_msg(1, lpp_addr.clone());

    let res = instantiate(deps.as_mut(), mock_env(), owner(), msg).unwrap();
    assert_eq!(1, res.messages.len());

    // it worked, let's query the state
    let res = query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap();
    let config_response: ConfigResponse = from_binary(&res).unwrap();
    let config = config_response.config;
    assert_eq!(1, config.lease_code_id);
    assert_eq!(lpp_addr, config.lpp_addr);
}

#[test]
fn test_update_config() {
    let mut deps = mock_deps_with_contracts([LPP_ADDR, TIMEALARMS_ADDR, PROFIT_ADDR, ORACLE_ADDR]);

    let expected_liability = Liability::new(
        Percent::from_percent(55),
        Percent::from_percent(5),
        Percent::from_percent(5),
        Percent::from_percent(1),
        Percent::from_percent(2),
        Percent::from_percent(1),
        Duration::from_hours(12),
    );
    let expected_repaiment =
        InterestPaymentSpec::new(Duration::from_secs(100), Duration::from_secs(10));
    setup_test_case(deps.as_mut());
    let msg = ExecuteMsg::Config {
        lease_interest_rate_margin: Percent::from_percent(5),
        liability: expected_liability,
        lease_interest_payment: expected_repaiment.clone(),
    };
    execute(deps.as_mut(), mock_env(), owner(), msg).unwrap();

    let config = query_config(deps.as_ref());
    assert_eq!(expected_liability, config.liability);
    assert_eq!(expected_repaiment, config.lease_interest_payment);
}

#[test]
#[should_panic(expected = "Healthy % should be < first liquidation %")]
fn test_update_config_invalid_liability() {
    let mut deps = mock_deps_with_contracts([LPP_ADDR, TIMEALARMS_ADDR, PROFIT_ADDR, ORACLE_ADDR]);

    #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
    #[serde(rename_all = "snake_case")]
    pub struct Liability {
        initial: Percent,
        healthy: Percent,
        max: Percent,
        first_liq_warn: Percent,
        second_liq_warn: Percent,
        third_liq_warn: Percent,
        recalc_time: Duration,
    }

    #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
    #[serde(rename_all = "snake_case")]
    pub enum MockExecuteMsg {
        Config {
            lease_interest_rate_margin: Percent,
            liability: Liability,
            lease_interest_payment: InterestPaymentSpec,
        },
        OpenLease {
            currency: String,
        },
    }

    let liability = Liability {
        initial: Percent::from_percent(55),
        healthy: Percent::from_percent(55),
        max: Percent::from_percent(55),
        first_liq_warn: Percent::from_percent(55),
        second_liq_warn: Percent::from_percent(55),
        third_liq_warn: Percent::from_percent(55),
        recalc_time: Duration::from_secs(100),
    };
    let mock_msg = MockExecuteMsg::Config {
        lease_interest_rate_margin: Percent::from_percent(5),
        liability,
        lease_interest_payment: InterestPaymentSpec::new(
            Duration::from_secs(20),
            Duration::from_secs(10),
        ),
    };

    let msg: ExecuteMsg = from_binary(&to_binary(&mock_msg).unwrap()).unwrap();

    setup_test_case(deps.as_mut());

    execute(deps.as_mut(), mock_env(), owner(), msg).unwrap();
}

#[test]
fn test_update_config_unauthorized() {
    let mut deps = mock_deps_with_contracts([LPP_ADDR, TIMEALARMS_ADDR, PROFIT_ADDR, ORACLE_ADDR]);

    let expected_liability = Liability::new(
        Percent::from_percent(55),
        Percent::from_percent(5),
        Percent::from_percent(5),
        Percent::from_percent(1),
        Percent::from_percent(2),
        Percent::from_percent(1),
        Duration::from_hours(12),
    );
    let expected_repaiment =
        InterestPaymentSpec::new(Duration::from_secs(12), Duration::from_secs(10));
    setup_test_case(deps.as_mut());
    let msg = ExecuteMsg::Config {
        lease_interest_rate_margin: Percent::from_percent(5),
        liability: expected_liability,
        lease_interest_payment: expected_repaiment,
    };

    let err = execute(deps.as_mut(), mock_env(), customer(), msg).unwrap_err();
    assert_eq!(ContractError::Unauthorized(Unauthorized), err);
}

#[test]
fn test_no_dex_setup() {
    let mut deps = mock_deps_with_contracts([LPP_ADDR, TIMEALARMS_ADDR, PROFIT_ADDR, ORACLE_ADDR]);

    setup_test_case(deps.as_mut());

    let config = query_config(deps.as_ref());
    assert!(config.dex.is_none());

    let msg = ExecuteMsg::OpenLease {
        currency: DENOM.to_string(),
    };

    let res = execute(deps.as_mut(), mock_env(), customer(), msg);
    assert_eq!(Err(ContractError::NoDEXConnectivitySetup {}), res);
}

#[test]
fn test_setup_dex_unauthorized() {
    let mut deps = mock_deps_with_contracts([LPP_ADDR, TIMEALARMS_ADDR, PROFIT_ADDR, ORACLE_ADDR]);

    setup_test_case(deps.as_mut());

    let res = setup_dex(deps.as_mut(), customer());
    assert_eq!(Err(ContractError::Unauthorized(Unauthorized)), res);
}

#[test]
fn test_setup_dex_again() {
    let mut deps = mock_deps_with_contracts([LPP_ADDR, TIMEALARMS_ADDR, PROFIT_ADDR, ORACLE_ADDR]);

    setup_test_case(deps.as_mut());

    setup_dex_ok(deps.as_mut());

    let res = setup_dex(deps.as_mut(), owner());
    assert_eq!(Err(ContractError::DEXConnectivityAlreadySetup {}), res);

    let res = setup_dex(deps.as_mut(), customer());
    assert_eq!(Err(ContractError::Unauthorized(Unauthorized)), res);
}

#[test]
fn test_open_lease() {
    let mut deps = mock_deps_with_contracts([LPP_ADDR, TIMEALARMS_ADDR, PROFIT_ADDR, ORACLE_ADDR]);

    setup_test_case(deps.as_mut());
    setup_dex_ok(deps.as_mut());

    let config = query_config(deps.as_ref());

    let msg = ExecuteMsg::OpenLease {
        currency: DENOM.to_string(),
    };
    let info = customer();
    let env = mock_env();
    let admin = env.contract.address.clone();
    let res = execute(deps.as_mut(), env, info.clone(), msg).unwrap();

    let msg = Borrow::open_lease_msg(info.sender, config, DENOM.to_string()).unwrap();
    assert_eq!(
        res.messages,
        vec![SubMsg::reply_on_success(
            CosmosMsg::Wasm(WasmMsg::Instantiate {
                funds: info.funds,
                msg: to_binary(&msg).unwrap(),
                admin: Some(admin.into()),
                code_id: 1,
                label: "lease".to_string()
            }),
            1
        )]
    );
}
