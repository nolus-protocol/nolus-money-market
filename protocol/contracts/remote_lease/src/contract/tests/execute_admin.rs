use platform::contract::{Code, CodeId};
use sdk::{
    cosmwasm_ext::{CosmosMsg, SubMsg},
    cosmwasm_std::{AnyMsg, IbcMsg, Uint64, testing},
};

use crate::{
    api::ExecuteMsg,
    contract::{execute, instantiate},
    error::Error,
    state::Channel,
};

use super::{
    ADMIN, LEASE_CODE_ID, LOCAL_CHANNEL_ID, NON_ADMIN, deps, instantiate_default, instantiate_msg,
    query_channel, query_config, sender, store_closing_channel, store_open_channel,
};

#[test]
fn new_lease_code_admin_succeeds() {
    let mut deps = deps();
    instantiate(
        deps.as_mut(),
        testing::mock_env(),
        sender(super::CREATOR),
        instantiate_msg(),
    )
    .unwrap();

    let new_code = Code::unchecked(LEASE_CODE_ID + 5);
    let res = execute(
        deps.as_mut(),
        testing::mock_env(),
        sender(ADMIN),
        ExecuteMsg::NewLeaseCode {
            lease_code: new_code,
        },
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
        sender(super::CREATOR),
        instantiate_msg(),
    )
    .unwrap();

    let err = execute(
        deps.as_mut(),
        testing::mock_env(),
        sender(NON_ADMIN),
        ExecuteMsg::NewLeaseCode {
            lease_code: Code::unchecked(LEASE_CODE_ID + 1),
        },
    )
    .unwrap_err();
    assert!(matches!(err, Error::Unauthorized(_)), "got {err:?}");

    let config = query_config(deps.as_ref());
    assert_eq!(Uint64::from(LEASE_CODE_ID), config.lease_code_id);
}
