use platform::contract::{Code, CodeId};
use sdk::{
    cosmwasm_ext::CosmosMsg,
    cosmwasm_std::{
        AnyMsg, Deps, DepsMut, IbcMsg, MessageInfo, OwnedDeps, SubMsg, Uint64,
        testing::{self, MockApi, MockQuerier, MockStorage},
    },
    testing as sdk_testing,
};

use crate::{
    api::{ChannelResponse, ConfigResponse, ExecuteMsg, InstantiateMsg, QueryMsg},
    contract::{execute, instantiate, query},
    error::Error,
    state::Channel,
};

const ADMIN: &str = "admin";
const NON_ADMIN: &str = "intruder";
const CREATOR: &str = "creator";
const CONNECTION_ID: &str = "connection-3";
const DEX_LABEL: &str = "osmosis";
const LEASE_CODE_ID: u64 = 17;
const LOCAL_CHANNEL_ID: &str = "channel-0";
const COUNTERPARTY_CHANNEL_ID: &str = "channel-77";
const COUNTERPARTY_PORT_ID: &str = "nls-remote-lease.osmosis";
const VERSION: &str = "nls-remote-lease.v1";

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
fn open_channel_admin_emits_any_msg() {
    const CHANNEL_OPEN_INIT: &str = "/ibc.core.channel.v1.MsgChannelOpenInit";

    let mut deps = deps();
    instantiate_default(deps.as_mut());

    let res = execute(
        deps.as_mut(),
        testing::mock_env(),
        sender(ADMIN),
        ExecuteMsg::OpenChannel(),
    )
    .unwrap();
    assert_eq!(1, res.messages.len());
    match &res.messages[0] {
        SubMsg {
            msg: CosmosMsg::Any(AnyMsg { type_url, value }),
            ..
        } => {
            assert_eq!(CHANNEL_OPEN_INIT, type_url);
            assert!(!value.is_empty());
        }
        other => panic!("expected CosmosMsg::Any, got {other:?}"),
    }

    assert!(Channel::may_load(&deps.storage).unwrap().is_none());
}

#[test]
fn open_channel_non_admin_rejected() {
    let mut deps = deps();
    instantiate_default(deps.as_mut());
    let err = execute(
        deps.as_mut(),
        testing::mock_env(),
        sender(NON_ADMIN),
        ExecuteMsg::OpenChannel(),
    )
    .unwrap_err();
    assert!(matches!(err, Error::Unauthorized(_)), "got {err:?}");
}

#[test]
fn open_channel_when_channel_exists_rejected() {
    let mut deps = deps();
    instantiate_default(deps.as_mut());
    store_open_channel(deps.as_mut());

    let err = execute(
        deps.as_mut(),
        testing::mock_env(),
        sender(ADMIN),
        ExecuteMsg::OpenChannel(),
    )
    .unwrap_err();
    assert!(matches!(err, Error::ChannelAlreadyExists), "got {err:?}");
}

#[test]
fn close_channel_admin_transitions_state_and_emits_close() {
    let mut deps = deps();
    instantiate_default(deps.as_mut());
    store_open_channel(deps.as_mut());

    let res = execute(
        deps.as_mut(),
        testing::mock_env(),
        sender(ADMIN),
        ExecuteMsg::CloseChannel(),
    )
    .unwrap();
    assert_eq!(1, res.messages.len());
    assert!(matches!(
        &res.messages[0].msg,
        CosmosMsg::Ibc(IbcMsg::CloseChannel { channel_id }) if channel_id == LOCAL_CHANNEL_ID
    ));

    let channel = query_channel(deps.as_ref()).channel.unwrap();
    assert!(matches!(
        channel.state,
        crate::api::ChannelStateResponse::Closing
    ));
}

#[test]
fn close_channel_non_admin_rejected() {
    let mut deps = deps();
    instantiate_default(deps.as_mut());
    store_open_channel(deps.as_mut());

    let err = execute(
        deps.as_mut(),
        testing::mock_env(),
        sender(NON_ADMIN),
        ExecuteMsg::CloseChannel(),
    )
    .unwrap_err();
    assert!(matches!(err, Error::Unauthorized(_)), "got {err:?}");
}

#[test]
fn close_channel_when_absent_rejected() {
    let mut deps = deps();
    instantiate_default(deps.as_mut());

    let err = execute(
        deps.as_mut(),
        testing::mock_env(),
        sender(ADMIN),
        ExecuteMsg::CloseChannel(),
    )
    .unwrap_err();
    assert!(matches!(err, Error::ChannelNotOpen), "got {err:?}");
}

#[test]
fn close_channel_when_already_closing_rejected() {
    let mut deps = deps();
    instantiate_default(deps.as_mut());
    store_closing_channel(deps.as_mut());

    let err = execute(
        deps.as_mut(),
        testing::mock_env(),
        sender(ADMIN),
        ExecuteMsg::CloseChannel(),
    )
    .unwrap_err();
    assert!(matches!(err, Error::ChannelNotOperational), "got {err:?}");
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

fn instantiate_default(deps: DepsMut<'_>) {
    instantiate(
        deps,
        testing::mock_env(),
        sender(CREATOR),
        instantiate_msg(),
    )
    .unwrap();
}

fn store_open_channel(deps: DepsMut<'_>) {
    Channel::new_open(
        LOCAL_CHANNEL_ID.into(),
        COUNTERPARTY_CHANNEL_ID.into(),
        COUNTERPARTY_PORT_ID.into(),
        VERSION.into(),
    )
    .store(deps.storage)
    .unwrap();
}

fn store_closing_channel(deps: DepsMut<'_>) {
    Channel::new_open(
        LOCAL_CHANNEL_ID.into(),
        COUNTERPARTY_CHANNEL_ID.into(),
        COUNTERPARTY_PORT_ID.into(),
        VERSION.into(),
    )
    .into_closing()
    .unwrap()
    .store(deps.storage)
    .unwrap();
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
