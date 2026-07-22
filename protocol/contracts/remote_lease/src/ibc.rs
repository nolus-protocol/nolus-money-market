use currencies::{LeaseGroup, Lpns as LpnGroup, PaymentGroup};
use remote_lease::{
    callback::{RemoteErrorMessage, RemoteLeaseCallback},
    envelope::{NolusLeaseAddr, PacketEnvelope},
    response::OperationResponse,
};
use sdk::{
    cosmos_sdk_proto::prost::Message as _,
    cosmwasm_ext::CosmosMsg,
    cosmwasm_std::{
        self, Addr, AnyMsg, Api, Binary, DepsMut, Env, IbcBasicResponse, IbcChannel,
        IbcChannelCloseMsg, IbcChannelConnectMsg, IbcChannelOpenMsg, IbcChannelOpenResponse,
        IbcMsg, IbcOrder, IbcPacketAckMsg, IbcPacketReceiveMsg, IbcPacketTimeoutMsg,
        IbcReceiveResponse, Never, StdAck, WasmMsg, entry_point,
    },
    ibc_proto::ibc::core::channel::v1::{
        Channel as ProtoChannel, Counterparty as ProtoCounterparty, MsgChannelOpenInit,
        Order as ProtoOrder, State as ProtoState,
    },
};

use crate::{
    error::{Error, Result},
    lease_callback::LeaseExecuteMsg,
    state::{Channel, ChannelState, Config},
};

const MSG_CHANNEL_OPEN_INIT_TYPE_URL: &str = "/ibc.core.channel.v1.MsgChannelOpenInit";

/// Build the `CosmosMsg::Any { MsgChannelOpenInit }` that initiates the handshake.
pub fn build_channel_open_init(env: &Env, config: &Config) -> CosmosMsg {
    let counterparty_port_id = remote_lease::port_id_for(config.dex_label());
    let channel = ProtoChannel {
        state: ProtoState::Init.into(),
        ordering: ProtoOrder::Unordered.into(),
        counterparty: Some(ProtoCounterparty {
            port_id: counterparty_port_id,
            channel_id: String::new(),
        }),
        connection_hops: vec![config.connection_id().to_string()],
        version: remote_lease::VERSION.to_string(),
        upgrade_sequence: 0,
    };
    let msg = MsgChannelOpenInit {
        port_id: contract_port_id(&env.contract.address),
        channel: Some(channel),
        signer: env.contract.address.to_string(),
    };

    CosmosMsg::Any(AnyMsg {
        type_url: MSG_CHANNEL_OPEN_INIT_TYPE_URL.to_string(),
        value: Binary::new(msg.encode_to_vec()),
    })
}

/// Build the `CosmosMsg::Ibc(IbcMsg::CloseChannel)` for the recorded local channel.
pub fn build_channel_close(channel: &Channel) -> CosmosMsg {
    CosmosMsg::Ibc(IbcMsg::CloseChannel {
        channel_id: channel.local_channel_id().to_string(),
    })
}

#[entry_point]
pub fn ibc_channel_open(
    deps: DepsMut<'_>,
    _env: Env,
    msg: IbcChannelOpenMsg,
) -> Result<IbcChannelOpenResponse> {
    match msg {
        IbcChannelOpenMsg::OpenInit { channel } => Config::load(deps.storage)
            .and_then(|config| validate_handshake_channel(&channel, &config))
            .map(|()| None),
        IbcChannelOpenMsg::OpenTry { .. } => Err(Error::UnsupportedCounterpartyOpen),
    }
}

#[entry_point]
pub fn ibc_channel_connect(
    deps: DepsMut<'_>,
    _env: Env,
    msg: IbcChannelConnectMsg,
) -> Result<IbcBasicResponse> {
    let channel = match msg {
        IbcChannelConnectMsg::OpenAck { channel, .. }
        | IbcChannelConnectMsg::OpenConfirm { channel } => channel,
    };

    Channel::may_load(deps.storage)
        .and_then(|existing| match existing {
            Some(_) => Err(Error::ChannelAlreadyExists),
            None => Config::load(deps.storage)
                .and_then(|config| validate_handshake_channel(&channel, &config).map(|()| channel)),
        })
        .and_then(|channel| persist_open_channel(deps, channel))
        .map(|()| IbcBasicResponse::new())
}

#[entry_point]
pub fn ibc_channel_close(
    deps: DepsMut<'_>,
    _env: Env,
    msg: IbcChannelCloseMsg,
) -> Result<IbcBasicResponse> {
    match msg {
        IbcChannelCloseMsg::CloseInit { .. } => Channel::may_load(deps.storage)
            .and_then(|maybe_channel| match maybe_channel {
                Some(channel) if channel.state() == ChannelState::Closing => Ok(()),
                _ => Err(Error::UnsolicitedChannelClose),
            })
            .map(|()| IbcBasicResponse::new()),
        IbcChannelCloseMsg::CloseConfirm { .. } => {
            Channel::clear(deps.storage);
            Ok(IbcBasicResponse::new())
        }
    }
}

#[entry_point]
pub fn ibc_packet_receive(
    _deps: DepsMut<'_>,
    _env: Env,
    _msg: IbcPacketReceiveMsg,
) -> std::result::Result<IbcReceiveResponse, Never> {
    Ok(IbcReceiveResponse::new(
        StdAck::error(Error::UnsupportedInboundPacket.to_string()).to_binary(),
    ))
}

#[entry_point]
pub fn ibc_packet_ack(
    deps: DepsMut<'_>,
    _env: Env,
    msg: IbcPacketAckMsg,
) -> Result<IbcBasicResponse> {
    cosmwasm_std::from_json(&msg.original_packet.data)
        .map_err(Error::from)
        .and_then(|envelope| {
            cosmwasm_std::from_json::<StdAck>(&msg.acknowledgement.data)
                .map_err(Error::from)
                .and_then(ack_to_callback)
                .and_then(|callback| dispatch_lease_callback(deps.api, envelope, callback))
        })
}

#[entry_point]
pub fn ibc_packet_timeout(
    deps: DepsMut<'_>,
    _env: Env,
    msg: IbcPacketTimeoutMsg,
) -> Result<IbcBasicResponse> {
    cosmwasm_std::from_json(&msg.packet.data)
        .map_err(Error::from)
        .and_then(|envelope| {
            dispatch_lease_callback(deps.api, envelope, RemoteLeaseCallback::OperationTimeout)
        })
}

fn ack_to_callback(ack: StdAck) -> Result<RemoteLeaseCallback<PaymentGroup>> {
    match ack {
        StdAck::Success(data) => cosmwasm_std::from_json::<OperationResponse<PaymentGroup>>(&data)
            .map(RemoteLeaseCallback::OperationOk)
            .map_err(Error::from),
        StdAck::Error(message) => RemoteErrorMessage::new(message)
            .map(RemoteLeaseCallback::OperationErr)
            .map_err(Error::from),
    }
}

// Trust model for `envelope.lease`: `into_validated` checks format only — the
// returned `Addr` is not re-checked against `Config.lease_code`. The address
// was placed in `original_packet.data` by this controller at send-time
// (`contract::send_operation` → `auth_caller`), and ibc-go commits packet
// bytes on-chain at send-time, so the inbound bytes are tamper-resistant by
// the light-client. Per ADR 0001 §5 identity flows from the light client +
// port uniqueness, not from a per-packet whitelist.
fn dispatch_lease_callback(
    api: &dyn Api,
    envelope: PacketEnvelope<LeaseGroup, LpnGroup, PaymentGroup>,
    callback: RemoteLeaseCallback<PaymentGroup>,
) -> Result<IbcBasicResponse> {
    envelope
        .lease
        .into_validated(api)
        .map_err(Error::from)
        .and_then(|lease_addr| {
            cosmwasm_std::to_json_binary(&LeaseExecuteMsg::RemoteLeaseCallback(callback))
                .map_err(Error::from)
                .map(|msg| WasmMsg::Execute {
                    contract_addr: lease_addr.into_string(),
                    msg,
                    funds: vec![],
                })
        })
        .map(|wasm_msg| IbcBasicResponse::new().add_message(wasm_msg))
}

fn validate_handshake_channel(channel: &IbcChannel, config: &Config) -> Result<()> {
    require_unordered(channel.order.clone())
        .and_then(|()| require_version(&channel.version))
        .and_then(|()| require_connection_id(&channel.connection_id, config.connection_id()))
        .and_then(|()| {
            require_counterparty_port(&channel.counterparty_endpoint.port_id, config.dex_label())
        })
}

fn require_unordered(order: IbcOrder) -> Result<()> {
    match order {
        IbcOrder::Unordered => Ok(()),
        IbcOrder::Ordered => Err(Error::InvalidChannelOrdering),
    }
}

fn require_version(actual: &str) -> Result<()> {
    if actual == remote_lease::VERSION {
        Ok(())
    } else {
        Err(Error::InvalidChannelVersion {
            expected: remote_lease::VERSION.to_string(),
            actual: actual.to_string(),
        })
    }
}

fn require_connection_id(actual: &str, expected: &str) -> Result<()> {
    if actual == expected {
        Ok(())
    } else {
        Err(Error::InvalidConnectionId {
            expected: expected.to_string(),
            actual: actual.to_string(),
        })
    }
}

fn require_counterparty_port(actual: &str, dex_label: &str) -> Result<()> {
    let expected = remote_lease::port_id_for(dex_label);
    if actual == expected {
        Ok(())
    } else {
        Err(Error::InvalidCounterpartyPort {
            expected,
            actual: actual.to_string(),
        })
    }
}

fn persist_open_channel(deps: DepsMut<'_>, channel: IbcChannel) -> Result<()> {
    let IbcChannel {
        endpoint,
        counterparty_endpoint,
        version,
        ..
    } = channel;
    Channel::new_open(
        endpoint.channel_id,
        counterparty_endpoint.channel_id,
        counterparty_endpoint.port_id,
        version,
    )
    .store(deps.storage)
}

fn contract_port_id(contract: &Addr) -> String {
    format!("wasm.{contract}")
}

#[cfg(test)]
mod tests;
