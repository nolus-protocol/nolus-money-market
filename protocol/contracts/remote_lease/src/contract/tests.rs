use platform::contract::{Code, CodeId};
use sdk::{
    cosmwasm_std::{
        Deps, MessageInfo, OwnedDeps, Uint64,
        testing::{self, MockApi, MockQuerier, MockStorage},
    },
    testing as sdk_testing,
};

use crate::{
    api::{ChannelResponse, ConfigResponse, ExecuteMsg, InstantiateMsg, QueryMsg},
    contract::{execute, instantiate, query},
    error::Error,
};

const ADMIN: &str = "admin";
const NON_ADMIN: &str = "intruder";
const CREATOR: &str = "creator";
const CONNECTION_ID: &str = "connection-3";
const DEX_LABEL: &str = "osmosis";
const LEASE_CODE_ID: u64 = 17;

#[test]
fn proper_initialization() {
    let mut deps = deps();
    let res = instantiate(
        deps.as_mut(),
        testing::mock_env(),
        sender(CREATOR),
        instantiate_msg(),
    )
    .unwrap();
    assert_eq!(0, res.messages.len());

    let config = query_config(deps.as_ref());
    assert_eq!(CONNECTION_ID, config.connection_id);
    assert_eq!(DEX_LABEL, config.dex_label);
    assert_eq!(
        Uint64::from(CodeId::from(Code::unchecked(LEASE_CODE_ID))),
        config.lease_code_id,
    );

    let channel = query_channel(deps.as_ref());
    assert_eq!(None, channel.channel);
}

#[test]
fn instantiate_rejects_empty_connection_id() {
    let mut deps = deps();
    let msg = InstantiateMsg {
        connection_id: String::new(),
        ..instantiate_msg()
    };
    let err = instantiate(deps.as_mut(), testing::mock_env(), sender(CREATOR), msg).unwrap_err();
    assert!(
        matches!(err, Error::EmptyInstantiateField("connection_id")),
        "got {err:?}"
    );
}

#[test]
fn instantiate_rejects_empty_dex_label() {
    let mut deps = deps();
    let msg = InstantiateMsg {
        dex_label: String::new(),
        ..instantiate_msg()
    };
    let err = instantiate(deps.as_mut(), testing::mock_env(), sender(CREATOR), msg).unwrap_err();
    assert!(
        matches!(err, Error::EmptyInstantiateField("dex_label")),
        "got {err:?}"
    );
}

#[test]
fn instantiate_rejects_malformed_admin() {
    let mut deps = deps();
    let msg = InstantiateMsg {
        protocol_admin: "NOT_BECH32!".into(),
        ..instantiate_msg()
    };
    let err = instantiate(deps.as_mut(), testing::mock_env(), sender(CREATOR), msg).unwrap_err();
    assert!(matches!(err, Error::Std(_)), "got {err:?}");
}

#[test]
fn new_lease_code_admin_succeeds() {
    let mut deps = deps();
    instantiate(
        deps.as_mut(),
        testing::mock_env(),
        sender(CREATOR),
        instantiate_msg(),
    )
    .unwrap();

    let new_code = Code::unchecked(LEASE_CODE_ID + 5);
    let res = execute(
        deps.as_mut(),
        testing::mock_env(),
        sender(ADMIN),
        ExecuteMsg::NewLeaseCode(new_code),
    )
    .unwrap();
    assert_eq!(0, res.messages.len());

    let config = query_config(deps.as_ref());
    assert_eq!(Uint64::from(CodeId::from(new_code)), config.lease_code_id);
}

#[test]
fn new_lease_code_non_admin_rejected() {
    let mut deps = deps();
    instantiate(
        deps.as_mut(),
        testing::mock_env(),
        sender(CREATOR),
        instantiate_msg(),
    )
    .unwrap();

    let err = execute(
        deps.as_mut(),
        testing::mock_env(),
        sender(NON_ADMIN),
        ExecuteMsg::NewLeaseCode(Code::unchecked(LEASE_CODE_ID + 1)),
    )
    .unwrap_err();
    assert!(matches!(err, Error::Unauthorized(_)), "got {err:?}");

    let config = query_config(deps.as_ref());
    assert_eq!(Uint64::from(LEASE_CODE_ID), config.lease_code_id);
}

fn deps() -> OwnedDeps<MockStorage, MockApi, MockQuerier> {
    sdk_testing::mock_deps_with_contracts([])
}

fn instantiate_msg() -> InstantiateMsg {
    InstantiateMsg {
        protocol_admin: sdk_testing::user(ADMIN).into_string(),
        connection_id: CONNECTION_ID.into(),
        dex_label: DEX_LABEL.into(),
        lease_code: LEASE_CODE_ID.into(),
    }
}

fn sender(who: &str) -> MessageInfo {
    MessageInfo {
        sender: sdk_testing::user(who),
        funds: vec![],
    }
}

fn query_config(deps: Deps<'_>) -> ConfigResponse {
    let raw = query(deps, testing::mock_env(), QueryMsg::Config()).unwrap();
    sdk::cosmwasm_std::from_json(raw).unwrap()
}

fn query_channel(deps: Deps<'_>) -> ChannelResponse {
    let raw = query(deps, testing::mock_env(), QueryMsg::Channel()).unwrap();
    sdk::cosmwasm_std::from_json(raw).unwrap()
}
