use sdk::{
    cosmos_sdk_proto::prost::Message as _,
    cosmwasm_ext::CosmosMsg,
    cosmwasm_std::{
        Addr, AnyMsg, Binary, DepsMut, Env, IbcBasicResponse, IbcChannel, IbcChannelCloseMsg,
        IbcChannelConnectMsg, IbcChannelOpenMsg, IbcChannelOpenResponse, IbcMsg, IbcOrder,
        IbcPacketAckMsg, IbcPacketReceiveMsg, IbcPacketTimeoutMsg, IbcReceiveResponse, Never,
        StdAck, entry_point,
    },
    ibc_proto::ibc::core::channel::v1::{
        Channel as ProtoChannel, Counterparty as ProtoCounterparty, MsgChannelOpenInit,
        Order as ProtoOrder, State as ProtoState,
    },
};

use crate::{
    error::{Error, Result},
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
    _deps: DepsMut<'_>,
    _env: Env,
    _msg: IbcPacketAckMsg,
) -> Result<IbcBasicResponse> {
    Ok(IbcBasicResponse::new())
}

#[entry_point]
pub fn ibc_packet_timeout(
    _deps: DepsMut<'_>,
    _env: Env,
    _msg: IbcPacketTimeoutMsg,
) -> Result<IbcBasicResponse> {
    Ok(IbcBasicResponse::new())
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
