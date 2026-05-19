use currencies::{
    PaymentGroup,
    testing::{PaymentC1, PaymentC2, PaymentC3},
};
use finance::{coin::Coin, duration::Duration, instant::Instant};
use platform::contract::{Code, CodeId};
use remote_lease::{
    callback::RemoteLeaseCallback,
    envelope::{LeaseAddrOnWire, PacketEnvelope},
    msg::{CloseLeaseParams, OpenLeaseParams, Operation, SwapParams, TransferOutParams},
    response::{OpenLeaseResponse, OperationResponse},
    version::ProtocolVersion,
};
use sdk::{
    cosmwasm_ext::{CosmosMsg, SubMsg},
    cosmwasm_std::{
        self, Addr, AnyMsg, Binary, ContractInfoResponse, ContractResult, Deps, DepsMut,
        IbcAcknowledgement, IbcChannel, IbcChannelCloseMsg, IbcChannelConnectMsg, IbcEndpoint,
        IbcMsg, IbcOrder, IbcPacket, IbcPacketAckMsg, IbcTimeout, MessageInfo, OwnedDeps, StdAck,
        SubMsg as StdSubMsg, SystemError, SystemResult, Timestamp, Uint64, WasmMsg, WasmQuery,
        testing::{self, MockApi, MockQuerier, MockStorage},
    },
    testing as sdk_testing,
};
use versioning::{
    ProtocolMigrationMessage, ProtocolPackageRelease, ProtocolPackageReleaseId, ReleaseId,
    VersionSegment, package_name, package_version,
};

use crate::{
    api::{
        ChannelResponse, ChannelStateResponse, ConfigResponse, ExecuteMsg, InstantiateMsg,
        MigrateMsg, QueryMsg,
    },
    contract::{execute, instantiate, migrate, query},
    error::Error,
    ibc::{ibc_channel_close, ibc_channel_connect, ibc_packet_ack},
    lease_callback::LeaseExecuteMsg,
    state::Channel,
};

const ADMIN: &str = "admin";
const NON_ADMIN: &str = "intruder";
const CREATOR: &str = "creator";
const CONNECTION_ID: &str = "connection-3";
const DEX_LABEL: &str = "osmosis";
const LEASE_CODE_ID: u64 = 17;
const WRONG_CODE_ID: u64 = LEASE_CODE_ID + 1;
const LEASE: &str = "lease";
const WRONG_CODE_CONTRACT: &str = "wrong-code-contract";
const NON_CONTRACT_CALLER: &str = "wallet-only";
const PACKET_TIMEOUT: Duration = Duration::from_secs(600);
const LOCAL_CHANNEL_ID: &str = "channel-0";
const COUNTERPARTY_CHANNEL_ID: &str = "channel-77";
const COUNTERPARTY_PORT_ID: &str = "nls-remote-lease.osmosis";
const VERSION: &str = "nls-remote-lease.v1";
const CONTRACT_STORAGE_VERSION: VersionSegment = 0;

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
fn instantiate_rejects_unknown_lease_code() {
    let mut deps = deps_with_failing_code_info();
    let err = instantiate(
        deps.as_mut(),
        testing::mock_env(),
        sender(CREATOR),
        instantiate_msg(),
    )
    .unwrap_err();
    assert!(matches!(err, Error::Platform(_)), "got {err:?}");
}

#[test]
fn migrate_same_release_succeeds() {
    let mut deps = deps();
    instantiate_default(deps.as_mut());

    let res = migrate(deps.as_mut(), testing::mock_env(), migrate_msg()).unwrap();
    assert_eq!(0, res.messages.len());
}

#[test]
fn migrate_mismatched_to_release_id_propagates_update_software_error() {
    let mut deps = deps();
    instantiate_default(deps.as_mut());
    let mut msg = migrate_msg();
    msg.to_release = ProtocolPackageReleaseId::new(
        ReleaseId::new_test("not-the-build-id"),
        ReleaseId::new_test("not-the-build-id"),
    );
    let err = migrate(deps.as_mut(), testing::mock_env(), msg).unwrap_err();
    assert!(matches!(err, Error::UpdateSoftware(_)), "got {err:?}");
}

#[test]
fn query_protocol_package_release_returns_current() {
    let deps = deps();
    let raw = query(
        deps.as_ref(),
        testing::mock_env(),
        QueryMsg::ProtocolPackageRelease {},
    )
    .unwrap();
    let parsed: ProtocolPackageRelease = sdk::cosmwasm_std::from_json(raw).unwrap();
    let expected = ProtocolPackageRelease::current(
        package_name!(),
        package_version!(),
        CONTRACT_STORAGE_VERSION,
    );
    assert_eq!(
        sdk::cosmwasm_std::to_json_binary(&expected).unwrap(),
        sdk::cosmwasm_std::to_json_binary(&parsed).unwrap(),
    );
}

#[test]
fn query_channel_returns_open_state_when_channel_is_open() {
    let mut deps = deps();
    instantiate_default(deps.as_mut());
    store_open_channel(deps.as_mut());

    let info = query_channel(deps.as_ref())
        .channel
        .expect("an open channel is recorded");
    assert!(matches!(info.state, ChannelStateResponse::Open));
    assert_eq!(LOCAL_CHANNEL_ID, info.local_channel_id);
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
        sender(CREATOR),
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

#[test]
fn open_lease_emits_send_packet() {
    let mut deps = deps_with_lease();
    instantiate_default(deps.as_mut());
    store_open_channel(deps.as_mut());

    let params = sample_open_lease_params();
    let res = execute(
        deps.as_mut(),
        testing::mock_env(),
        sender(LEASE),
        ExecuteMsg::OpenLease {
            params: params.clone(),
            timeout: PACKET_TIMEOUT,
        },
    )
    .unwrap();
    assert_send_packet(&Operation::OpenLease(params), &res.messages);
}

#[test]
fn close_lease_emits_send_packet() {
    let mut deps = deps_with_lease();
    instantiate_default(deps.as_mut());
    store_open_channel(deps.as_mut());

    let params = CloseLeaseParams {};
    let res = execute(
        deps.as_mut(),
        testing::mock_env(),
        sender(LEASE),
        ExecuteMsg::CloseLease {
            params: params.clone(),
            timeout: PACKET_TIMEOUT,
        },
    )
    .unwrap();
    assert_send_packet(&Operation::CloseLease(params), &res.messages);
}

#[test]
fn swap_emits_send_packet() {
    let mut deps = deps_with_lease();
    instantiate_default(deps.as_mut());
    store_open_channel(deps.as_mut());

    let params = sample_swap_params();
    let res = execute(
        deps.as_mut(),
        testing::mock_env(),
        sender(LEASE),
        ExecuteMsg::Swap {
            params: params.clone(),
            timeout: PACKET_TIMEOUT,
        },
    )
    .unwrap();
    assert_send_packet(&Operation::Swap(params), &res.messages);
}

#[test]
fn transfer_out_emits_send_packet() {
    let mut deps = deps_with_lease();
    instantiate_default(deps.as_mut());
    store_open_channel(deps.as_mut());

    let params = sample_transfer_out_params();
    let res = execute(
        deps.as_mut(),
        testing::mock_env(),
        sender(LEASE),
        ExecuteMsg::TransferOut {
            params: params.clone(),
            timeout: PACKET_TIMEOUT,
        },
    )
    .unwrap();
    assert_send_packet(&Operation::TransferOut(params), &res.messages);
}

#[test]
fn outbound_when_no_channel_rejected() {
    let mut deps = deps_with_lease();
    instantiate_default(deps.as_mut());

    let err = execute(
        deps.as_mut(),
        testing::mock_env(),
        sender(LEASE),
        open_lease_execute(),
    )
    .unwrap_err();
    assert!(matches!(err, Error::ChannelNotOpen), "got {err:?}");
}

#[test]
fn outbound_when_channel_closing_rejected() {
    let mut deps = deps_with_lease();
    instantiate_default(deps.as_mut());
    store_closing_channel(deps.as_mut());

    let err = execute(
        deps.as_mut(),
        testing::mock_env(),
        sender(LEASE),
        open_lease_execute(),
    )
    .unwrap_err();
    assert!(matches!(err, Error::ChannelNotOperational), "got {err:?}");
}

#[test]
fn outbound_wrong_caller_code_rejected() {
    let mut deps = deps_with_lease();
    instantiate_default(deps.as_mut());
    store_open_channel(deps.as_mut());

    let err = execute(
        deps.as_mut(),
        testing::mock_env(),
        sender(WRONG_CODE_CONTRACT),
        open_lease_execute(),
    )
    .unwrap_err();
    assert!(matches!(err, Error::UnauthorisedCaller), "got {err:?}");
}

#[test]
fn outbound_non_contract_caller_rejected() {
    let mut deps = deps_with_lease();
    instantiate_default(deps.as_mut());
    store_open_channel(deps.as_mut());

    let err = execute(
        deps.as_mut(),
        testing::mock_env(),
        sender(NON_CONTRACT_CALLER),
        open_lease_execute(),
    )
    .unwrap_err();
    assert!(matches!(err, Error::UnauthorisedCaller), "got {err:?}");
}

#[test]
fn scenario_open_channel_through_ack_dispatches_callback() {
    let mut deps = deps_with_lease();
    instantiate_default(deps.as_mut());
    open_channel_via_admin(deps.as_mut());
    drive_open_ack(deps.as_mut());

    let packet_data = drive_open_lease(deps.as_mut());

    let response = OperationResponse::OpenLease(OpenLeaseResponse {
        remote_lease_id: "sol-lease-7".into(),
    });
    let res = ibc_packet_ack(
        deps.as_mut(),
        testing::mock_env(),
        ack_msg_with(
            packet_data,
            StdAck::Success(cosmwasm_std::to_json_binary(&response).unwrap()).to_binary(),
        ),
    )
    .unwrap();

    assert_callback_to(
        &sdk_testing::user(LEASE),
        RemoteLeaseCallback::OperationOk(response),
        &res.messages,
    );
}

#[test]
fn scenario_close_channel_full_handshake_clears_state() {
    let mut deps = deps();
    instantiate_default(deps.as_mut());
    open_channel_via_admin(deps.as_mut());
    drive_open_ack(deps.as_mut());

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

    ibc_channel_close(
        deps.as_mut(),
        testing::mock_env(),
        IbcChannelCloseMsg::CloseConfirm {
            channel: handshake_channel(),
        },
    )
    .unwrap();

    assert!(Channel::may_load(&deps.storage).unwrap().is_none());
}

#[test]
fn scenario_unsolicited_close_init_while_open_rejected() {
    let mut deps = deps();
    instantiate_default(deps.as_mut());
    open_channel_via_admin(deps.as_mut());
    drive_open_ack(deps.as_mut());

    let err = ibc_channel_close(
        deps.as_mut(),
        testing::mock_env(),
        IbcChannelCloseMsg::CloseInit {
            channel: handshake_channel(),
        },
    )
    .unwrap_err();
    assert!(matches!(err, Error::UnsolicitedChannelClose), "got {err:?}");
}

fn deps() -> OwnedDeps<MockStorage, MockApi, MockQuerier> {
    sdk_testing::mock_deps_with_contracts([])
}

/// Wasm querier that resolves the two registered contract addresses to their
/// distinct code ids; all other addresses return `NoSuchContract`. CodeInfo is
/// passed through so `instantiate` can validate `lease_code` (see
/// `sdk_testing::mock_deps_with_contracts` for the equivalent default handling).
fn deps_with_lease() -> OwnedDeps<MockStorage, MockApi, MockQuerier> {
    let lease = sdk_testing::user(LEASE);
    let wrong = sdk_testing::user(WRONG_CODE_CONTRACT);
    let mut deps = sdk_testing::mock_deps_with_contracts([]);
    deps.querier.update_wasm(move |query| match query {
        WasmQuery::ContractInfo { contract_addr } => {
            let addr = Addr::unchecked(contract_addr);
            if addr == lease {
                contract_info_response(LEASE_CODE_ID)
            } else if addr == wrong {
                contract_info_response(WRONG_CODE_ID)
            } else {
                SystemResult::Err(SystemError::NoSuchContract {
                    addr: contract_addr.clone(),
                })
            }
        }
        WasmQuery::CodeInfo { code_id } => SystemResult::Ok(ContractResult::Ok(
            sdk::cosmwasm_std::to_json_binary(&sdk::cosmwasm_std::CodeInfoResponse::new(
                *code_id,
                sdk_testing::user(""),
                sdk::cosmwasm_std::Checksum::generate(&[0x1f, 0x4e, 0x20, 0x9a]),
            ))
            .expect("serialization succeeds"),
        )),
        _ => unimplemented!(),
    });
    deps
}

fn contract_info_response(code_id: u64) -> SystemResult<ContractResult<Binary>> {
    SystemResult::Ok(ContractResult::Ok(
        sdk::cosmwasm_std::to_json_binary(&ContractInfoResponse::new(
            code_id,
            sdk_testing::user("creator"),
            None,
            false,
            None,
            None,
        ))
        .expect("serialization succeeds"),
    ))
}

// Querier that returns `NoSuchCode` for every `CodeInfo` query. The contract's
// `Code::try_new` resolves to a `CodeInfo` query under the hood; replacing the
// default closure (which would unconditionally return a happy `CodeInfoResponse`)
// is the only way to exercise that error arm.
fn deps_with_failing_code_info() -> OwnedDeps<MockStorage, MockApi, MockQuerier> {
    let mut deps = sdk_testing::mock_deps_with_contracts([]);
    deps.querier.update_wasm(|query| match query {
        WasmQuery::CodeInfo { code_id } => {
            SystemResult::Err(SystemError::NoSuchCode { code_id: *code_id })
        }
        _ => unimplemented!("unexpected wasm query in this test"),
    });
    deps
}

fn migrate_msg() -> ProtocolMigrationMessage<MigrateMsg> {
    // Both env vars are supplied by `protocol/.cargo/config.toml` (see RUNBOOK
    // entry "Cargo / cargo config override entry for SOFTWARE_RELEASE_ID").
    // Running `cargo test` from outside the protocol workspace will fail to
    // compile this file with a missing-env-var error.
    const SOFTWARE_ID: &str = env!("SOFTWARE_RELEASE_ID");
    const PROTOCOL_ID: &str = env!("PROTOCOL_RELEASE_ID");
    let release = ProtocolPackageRelease::current(
        package_name!(),
        package_version!(),
        CONTRACT_STORAGE_VERSION,
    );
    ProtocolMigrationMessage {
        migrate_from: release,
        to_release: ProtocolPackageReleaseId::new(
            ReleaseId::new_test(SOFTWARE_ID),
            ReleaseId::new_test(PROTOCOL_ID),
        ),
        message: MigrateMsg {},
    }
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

fn open_lease_execute() -> ExecuteMsg {
    ExecuteMsg::OpenLease {
        params: sample_open_lease_params(),
        timeout: PACKET_TIMEOUT,
    }
}

fn sample_open_lease_params() -> OpenLeaseParams {
    OpenLeaseParams::new(
        7,
        currency::dto::<PaymentC1, PaymentGroup>(),
        currency::dto::<PaymentC2, PaymentGroup>(),
        currency::dto::<PaymentC3, PaymentGroup>(),
    )
    .expect("sample uses three distinct currencies")
}

fn sample_swap_params() -> SwapParams {
    SwapParams::new(
        Coin::<PaymentC1>::new(1_000).into(),
        Coin::<PaymentC2>::new(42).into(),
    )
    .expect("sample uses two distinct non-zero amounts")
}

fn sample_transfer_out_params() -> TransferOutParams {
    TransferOutParams::new(Coin::<PaymentC3>::new(1_000).into())
        .expect("sample uses a non-zero amount")
}

fn assert_send_packet(expected_operation: &Operation, messages: &[SubMsg]) {
    assert_eq!(1, messages.len(), "expected exactly one outbound message");
    match &messages[0].msg {
        CosmosMsg::Ibc(IbcMsg::SendPacket {
            channel_id,
            data,
            timeout,
        }) => {
            assert_eq!(LOCAL_CHANNEL_ID, channel_id);
            assert_eq!(&expected_timeout(), timeout);
            let envelope: PacketEnvelope = sdk::cosmwasm_std::from_json(data).unwrap();
            assert_eq!(
                LeaseAddrOnWire::new(sdk_testing::user(LEASE)),
                envelope.lease,
            );
            assert_eq!(expected_operation, &envelope.operation);
            assert_eq!(ProtocolVersion, envelope.version);
        }
        other => panic!("expected CosmosMsg::Ibc(IbcMsg::SendPacket {{..}}), got {other:?}"),
    }
}

fn expected_timeout() -> IbcTimeout {
    use cw_time::{IntoInstant as _, IntoTimestamp as _};
    let now: Instant = testing::mock_env().block.time.into_instant();
    IbcTimeout::with_timestamp((now + PACKET_TIMEOUT).into_timestamp())
}

fn open_channel_via_admin(deps: DepsMut<'_>) {
    let res = execute(
        deps,
        testing::mock_env(),
        sender(ADMIN),
        ExecuteMsg::OpenChannel(),
    )
    .expect("OpenChannel from admin must succeed");
    assert_eq!(1, res.messages.len());
}

fn drive_open_ack(deps: DepsMut<'_>) {
    ibc_channel_connect(
        deps,
        testing::mock_env(),
        IbcChannelConnectMsg::OpenAck {
            channel: handshake_channel(),
            counterparty_version: VERSION.into(),
        },
    )
    .expect("OpenAck must persist the channel");
}

fn drive_open_lease(deps: DepsMut<'_>) -> Binary {
    let params = sample_open_lease_params();
    let res = execute(
        deps,
        testing::mock_env(),
        sender(LEASE),
        ExecuteMsg::OpenLease {
            params,
            timeout: PACKET_TIMEOUT,
        },
    )
    .expect("OpenLease from authorised lease must succeed");
    match &res.messages[0].msg {
        CosmosMsg::Ibc(IbcMsg::SendPacket { data, .. }) => data.clone(),
        other => panic!("expected SendPacket, got {other:?}"),
    }
}

fn handshake_channel() -> IbcChannel {
    IbcChannel::new(
        local_endpoint(),
        IbcEndpoint {
            port_id: COUNTERPARTY_PORT_ID.into(),
            channel_id: COUNTERPARTY_CHANNEL_ID.into(),
        },
        IbcOrder::Unordered,
        VERSION,
        CONNECTION_ID,
    )
}

fn local_endpoint() -> IbcEndpoint {
    IbcEndpoint {
        port_id: format!("wasm.{}", testing::mock_env().contract.address),
        channel_id: LOCAL_CHANNEL_ID.into(),
    }
}

fn ack_msg_with(envelope_bytes: Binary, ack_bytes: Binary) -> IbcPacketAckMsg {
    const PACKET_SEQUENCE: u64 = 1;
    IbcPacketAckMsg::new(
        IbcAcknowledgement::new(ack_bytes),
        IbcPacket::new(
            envelope_bytes,
            local_endpoint(),
            IbcEndpoint {
                port_id: COUNTERPARTY_PORT_ID.into(),
                channel_id: COUNTERPARTY_CHANNEL_ID.into(),
            },
            PACKET_SEQUENCE,
            IbcTimeout::with_timestamp(Timestamp::from_seconds(1)),
        ),
        sdk_testing::user("relayer"),
    )
}

fn assert_callback_to(
    expected_lease: &Addr,
    expected_callback: RemoteLeaseCallback,
    messages: &[StdSubMsg],
) {
    assert_eq!(1, messages.len(), "expected one dispatched callback");
    match &messages[0].msg {
        cosmwasm_std::CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr,
            msg,
            funds,
        }) => {
            assert_eq!(expected_lease.as_str(), contract_addr);
            assert!(funds.is_empty(), "callback must carry no funds");
            let expected = cosmwasm_std::to_json_binary(&LeaseExecuteMsg::RemoteLeaseCallback(
                expected_callback,
            ))
            .expect("expected callback serialises");
            assert_eq!(&expected, msg);
        }
        other => panic!("expected WasmMsg::Execute, got {other:?}"),
    }
}
