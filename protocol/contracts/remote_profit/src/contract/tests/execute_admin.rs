use platform::contract::{Code, CodeId, external};
use sdk::{
    cosmos_sdk_proto::prost::Message as _,
    cosmwasm_ext::{CosmosMsg, SubMsg},
    cosmwasm_std::{AnyMsg, IbcMsg, SystemError, SystemResult, WasmQuery, testing},
    ibc_proto::ibc::core::channel::v1::MsgChannelOpenInit,
};

use crate::{
    api::ExecuteMsg,
    contract::{execute, instantiate},
    error::Error,
    state::Channel,
};

use super::{
    ADMIN, LOCAL_CHANNEL_ID, NON_ADMIN, PROFIT_CODE_ID, VERSION, deps, instantiate_default,
    instantiate_msg, query_channel, query_config, sender, store_closing_channel,
    store_open_channel,
};

#[test]
fn new_profit_code_admin_succeeds() {
    let mut deps = deps();
    instantiate(
        deps.as_mut(),
        testing::mock_env(),
        sender(super::CREATOR),
        instantiate_msg(),
    )
    .unwrap();

    let new_code = Code::unchecked(PROFIT_CODE_ID + 5);
    let res = execute(
        deps.as_mut(),
        testing::mock_env(),
        sender(ADMIN),
        ExecuteMsg::NewProfitCode {
            profit_code: new_code,
        },
    )
    .unwrap();
    assert_eq!(0, res.messages.len());

    let config = query_config(deps.as_ref());
    assert_eq!(
        external::Code::from(CodeId::from(new_code)),
        config.profit_code_id
    );
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
            let open_init =
                MsgChannelOpenInit::decode(value.as_slice()).expect("a valid protobuf payload");
            assert_eq!(
                VERSION,
                open_init.channel.expect("a channel must be set").version,
            );
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
fn new_profit_code_non_admin_rejected() {
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
        ExecuteMsg::NewProfitCode {
            profit_code: Code::unchecked(PROFIT_CODE_ID + 1),
        },
    )
    .unwrap_err();
    assert!(matches!(err, Error::Unauthorized(_)), "got {err:?}");

    let config = query_config(deps.as_ref());
    assert_eq!(external::Code::from(PROFIT_CODE_ID), config.profit_code_id);
}

#[test]
fn new_profit_code_rejects_unknown_code() {
    let mut deps = deps();
    instantiate(
        deps.as_mut(),
        testing::mock_env(),
        sender(super::CREATOR),
        instantiate_msg(),
    )
    .unwrap();

    // The rotated code id now resolves to no on-chain code, so the existence
    // check must reject it and leave the stored code untouched.
    deps.querier.update_wasm(|query| match query {
        WasmQuery::CodeInfo { code_id } => {
            SystemResult::Err(SystemError::NoSuchCode { code_id: *code_id })
        }
        _ => unimplemented!("unexpected wasm query in this test"),
    });

    let err = execute(
        deps.as_mut(),
        testing::mock_env(),
        sender(ADMIN),
        ExecuteMsg::NewProfitCode {
            profit_code: Code::unchecked(PROFIT_CODE_ID + 9),
        },
    )
    .unwrap_err();
    assert!(matches!(err, Error::Platform(_)), "got {err:?}");

    let config = query_config(deps.as_ref());
    assert_eq!(external::Code::from(PROFIT_CODE_ID), config.profit_code_id);
}
