use cosmwasm_std::{
    OwnedDeps,
    testing::{MockApi, MockQuerier, MockStorage},
};

use currencies::{LeaseGroup, Lpn, testing::LeaseC1};
use currency::{CurrencyDTO, CurrencyDef as _};
use dex::{ConnectionParams, Ics20Channel};
use finance::{duration::Duration, liability::Liability, percent::Percent};
use lease::api::{limits::MaxSlippage, open::PositionSpecDTO};
use platform::contract::{Code, CodeId};

use sdk::{
    cosmwasm_std::{
        Addr, CosmosMsg, Deps, DepsMut, MessageInfo, SubMsg, WasmMsg, coins, from_json, testing,
        to_json_binary,
    },
    testing as sdk_testing,
};

use crate::{
    cmd::Borrow,
    contract::{execute, instantiate, query, sudo},
    msg::{ConfigResponse, ExecuteMsg, NewConfig, QueryMsg, SudoMsg},
    state::config::Config,
    tests,
};

const CREATOR: &str = "creator";
const LPP_ADDR: &str = "test";
const TIMEALARMS_ADDR: &str = "timealarms";
const ORACLE_ADDR: &str = "oracle";
const PROFIT_ADDR: &str = "profit";
const RESERVE_ADDR: &str = "reserve";
const PROTOCOLS_REGISTRY_ADDR: &str = "protocols";
const LEASE_ADMIN: &str = "lease_admin";

fn lease_currency() -> CurrencyDTO<LeaseGroup> {
    currency::dto::<LeaseC1, _>()
}

const MARGIN_INTEREST_RATE: Percent = Percent::from_permille(30);

fn leaser_instantiate_msg(lease_code: Code, lpp: Addr) -> crate::msg::InstantiateMsg {
    crate::msg::InstantiateMsg {
        lease_code: CodeId::from(lease_code).into(),
        lpp,
        profit: sdk_testing::user(PROFIT_ADDR),
        reserve: sdk_testing::user(RESERVE_ADDR),
        time_alarms: sdk_testing::user(TIMEALARMS_ADDR),
        market_price_oracle: sdk_testing::user(ORACLE_ADDR),
        protocols_registry: sdk_testing::user(PROTOCOLS_REGISTRY_ADDR),
        lease_position_spec: PositionSpecDTO::new(
            Liability::new(
                Percent::from_percent(65),
                Percent::from_percent(70),
                Percent::from_percent(73),
                Percent::from_percent(75),
                Percent::from_percent(78),
                Percent::from_percent(80),
                Duration::from_hours(1),
            ),
            tests::lpn_coin(1000),
            tests::lpn_coin(10),
        ),
        lease_interest_rate_margin: MARGIN_INTEREST_RATE,
        lease_due_period: Duration::from_days(90),
        lease_max_slippage: MaxSlippage {
            liquidation: Percent::from_percent(20),
        },
        lease_admin: sdk_testing::user(LEASE_ADMIN),
        dex: dex_params(),
    }
}

fn owner() -> MessageInfo {
    MessageInfo {
        sender: sdk_testing::user(CREATOR),
        funds: vec![],
    }
}

fn customer() -> MessageInfo {
    MessageInfo {
        sender: sdk_testing::user("addr0000"),
        funds: coins(2, Lpn::dex()),
    }
}

fn setup_test_case(deps: DepsMut<'_>) {
    let lpp_addr = sdk_testing::user(LPP_ADDR);
    let msg = leaser_instantiate_msg(Code::unchecked(1), lpp_addr);

    let resp = instantiate(deps, testing::mock_env(), owner(), msg).unwrap();
    assert_eq!(0, resp.messages.len());
}

fn query_config(deps: Deps<'_>) -> Config {
    let res = query(deps, testing::mock_env(), QueryMsg::Config {}).unwrap();
    let config_response: ConfigResponse = from_json(res).unwrap();
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

#[test]
fn proper_initialization() {
    let mut deps = deps();

    let lpp_addr = sdk_testing::user(LPP_ADDR);
    let lease_code = Code::unchecked(1);
    let msg = leaser_instantiate_msg(lease_code, lpp_addr.clone());

    let res = instantiate(deps.as_mut(), testing::mock_env(), owner(), msg).unwrap();
    assert_eq!(0, res.messages.len());

    // it worked, let's query the state
    let res = query(deps.as_ref(), testing::mock_env(), QueryMsg::Config {}).unwrap();
    let config_response: ConfigResponse = from_json(res).unwrap();
    let config = config_response.config;
    assert_eq!(lease_code, config.lease_code);
    assert_eq!(lpp_addr, config.lpp);
}

#[test]
fn test_update_config() {
    let mut deps = deps();

    setup_test_case(deps.as_mut());

    let new_config = tests::new_config();
    let msg = SudoMsg::Config(new_config.clone());

    sudo(deps.as_mut(), testing::mock_env(), msg).unwrap();

    let config = query_config(deps.as_ref());
    assert_eq!(
        new_config,
        NewConfig {
            lease_due_period: config.lease_due_period,
            lease_interest_rate_margin: config.lease_interest_rate_margin,
            lease_max_slippage: config.lease_max_slippage,
            lease_position_spec: config.lease_position_spec,
        }
    );
}

fn open_lease_with(max_ltd: Option<Percent>) {
    let mut deps = deps();

    setup_test_case(deps.as_mut());

    let config = query_config(deps.as_ref());

    let msg = ExecuteMsg::OpenLease {
        currency: lease_currency(),
        max_ltd,
    };
    let info = customer();
    let env = testing::mock_env();
    let admin = env.contract.address.clone();
    let finalizer = admin.clone();
    let res = execute(deps.as_mut(), env, info.clone(), msg).unwrap();

    let msg = Borrow::open_lease_msg(info.sender, config, lease_currency(), max_ltd, finalizer);
    assert_eq!(
        res.messages,
        vec![SubMsg::reply_on_success(
            CosmosMsg::Wasm(WasmMsg::Instantiate {
                funds: info.funds,
                msg: to_json_binary(&msg).unwrap(),
                admin: Some(admin.into()),
                code_id: 1,
                label: "lease".to_string(),
            }),
            0,
        )]
    );
}

#[test]
fn test_open_lease() {
    open_lease_with(None);
}

#[test]
fn test_open_lease_with_max_ltd() {
    open_lease_with(None);
    open_lease_with(Some(Percent::from_percent(5)));
}

fn deps() -> OwnedDeps<MockStorage, MockApi, MockQuerier> {
    sdk_testing::mock_deps_with_contracts([
        sdk_testing::user(LPP_ADDR),
        sdk_testing::user(TIMEALARMS_ADDR),
        sdk_testing::user(PROFIT_ADDR),
        sdk_testing::user(ORACLE_ADDR),
    ])
}
