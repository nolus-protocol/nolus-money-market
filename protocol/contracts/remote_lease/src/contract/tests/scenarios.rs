use remote_lease::{
    callback::RemoteLeaseCallback,
    response::{OpenLeaseResponse, OperationResponse},
};
use sdk::{
    cosmwasm_ext::CosmosMsg,
    cosmwasm_std::{
        self, Addr, Binary, DepsMut, IbcAcknowledgement, IbcChannel, IbcChannelCloseMsg,
        IbcChannelConnectMsg, IbcEndpoint, IbcMsg, IbcOrder, IbcPacket, IbcPacketAckMsg,
        IbcTimeout, StdAck, SubMsg as StdSubMsg, Timestamp, WasmMsg, testing,
    },
    testing as sdk_testing,
};

use crate::{
    api::ExecuteMsg,
    contract::execute,
    error::Error,
    ibc::{ibc_channel_close, ibc_channel_connect},
    lease_callback::LeaseExecuteMsg,
    state::Channel,
};

use super::{
    ADMIN, CONNECTION_ID, COUNTERPARTY_CHANNEL_ID, COUNTERPARTY_PORT_ID, LEASE, LOCAL_CHANNEL_ID,
    PACKET_TIMEOUT, VERSION, deps, deps_with_lease, instantiate_default, sample_open_lease_params,
    sender,
};

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
    let res = crate::ibc::ibc_packet_ack(
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
