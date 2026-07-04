use remote_profit::{
    callback::{RemoteErrorMessage, RemoteOperationOutcome, RemoteProfitCallback},
    envelope::PacketEnvelope,
    response::WireOperationResponse,
};
use sdk::{
    cosmos_sdk_proto::prost::Message as _,
    cosmwasm_ext::CosmosMsg,
    cosmwasm_std::{
        self, Addr, AnyMsg, Binary, DepsMut, Env, IbcBasicResponse, IbcChannel, IbcChannelCloseMsg,
        IbcChannelConnectMsg, IbcChannelOpenMsg, IbcChannelOpenResponse, IbcMsg, IbcOrder,
        IbcPacketAckMsg, IbcPacketReceiveMsg, IbcPacketTimeoutMsg, IbcReceiveResponse, Never,
        StdAck, Storage, WasmMsg, entry_point,
    },
    ibc_proto::ibc::core::channel::v1::{
        Channel as ProtoChannel, Counterparty as ProtoCounterparty, MsgChannelOpenInit,
        Order as ProtoOrder, State as ProtoState,
    },
};

use crate::{
    error::{Error, Result},
    profit_callback::ProfitExecuteMsg,
    state::{Channel, ChannelState, Config},
};

const MSG_CHANNEL_OPEN_INIT_TYPE_URL: &str = "/ibc.core.channel.v1.MsgChannelOpenInit";
const VERSION_TRANSFER_KEY: &str = "+transfer=";
// Diagnostic bound on the counterparty-authored version echoed into the
// handshake error — a legitimate version is ~45 characters.
const COUNTERPARTY_VERSION_ECHO_MAX_CHARS: usize = 64;
// Synthetic `OperationErr` reason surfaced when a counterparty `StdAck::Success`
// payload does not decode to our wire response. Absorbing it — rather than
// erring — keeps an already-committed packet from being stranded behind an
// endless relayer retry.
const UNDECODABLE_ACK_REASON: &str = "undecodable success acknowledgement";

/// Compose the channel handshake version: the protocol version extended with
/// the paired Solana-side ICS-20 transfer channel (ADR-0002 §3.3).
///
/// Handshake grammar only — packets keep pinning the bare
/// [`remote_profit::VERSION`]; extending the shared constant would break packet
/// deserialization on both sides.
fn handshake_version(config: &Config) -> String {
    format!(
        "{version}{key}{channel}",
        version = remote_profit::VERSION,
        key = VERSION_TRANSFER_KEY,
        channel = config.transfer_channel()
    )
}

/// Build the `CosmosMsg::Any { MsgChannelOpenInit }` that initiates the handshake.
pub fn build_channel_open_init(env: &Env, config: &Config) -> CosmosMsg {
    let counterparty_port_id = remote_profit::port_id_for(config.dex_label());
    let channel = ProtoChannel {
        state: ProtoState::Init.into(),
        ordering: ProtoOrder::Unordered.into(),
        counterparty: Some(ProtoCounterparty {
            port_id: counterparty_port_id,
            channel_id: String::new(),
        }),
        connection_hops: vec![config.connection_id().to_string()],
        version: handshake_version(config),
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
    let (channel, may_counterparty_version) = match msg {
        IbcChannelConnectMsg::OpenAck {
            channel,
            counterparty_version,
        } => (channel, Some(counterparty_version)),
        // `OpenConfirm` is delivered only to the try side of a handshake and
        // carries no counterparty version; this controller rejects `OpenTry`,
        // so the arm is unreachable while counterparty-initiated opens stay
        // unsupported.
        IbcChannelConnectMsg::OpenConfirm { channel } => (channel, None),
    };

    Channel::may_load(deps.storage)
        .and_then(|existing| match existing {
            Some(_) => Err(Error::ChannelAlreadyExists),
            None => Config::load(deps.storage).and_then(|config| {
                validate_handshake_channel(&channel, &config)
                    .and_then(|()| {
                        validate_counterparty_version(may_counterparty_version.as_deref(), &config)
                    })
                    .map(|()| channel)
            }),
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
            .map(|()| {
                Channel::clear(deps.storage);
                IbcBasicResponse::new()
            }),
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
    cosmwasm_std::from_json::<PacketEnvelope>(&msg.original_packet.data)
        .map_err(Error::from)
        .and_then(|envelope| {
            cosmwasm_std::from_json::<StdAck>(&msg.acknowledgement.data)
                .map_err(Error::from)
                .map(ack_to_outcome)
                .and_then(|outcome| dispatch_profit_callback(deps.storage, envelope, outcome))
        })
}

#[entry_point]
pub fn ibc_packet_timeout(
    deps: DepsMut<'_>,
    _env: Env,
    msg: IbcPacketTimeoutMsg,
) -> Result<IbcBasicResponse> {
    cosmwasm_std::from_json::<PacketEnvelope>(&msg.packet.data)
        .map_err(Error::from)
        .and_then(|envelope| {
            // `OperationTimeout` is constructed LOCALLY here, on the timeout
            // entry point ONLY. It is never produced from a wire payload: the
            // ack path (`ack_to_outcome`) decodes only `StdAck::Success` /
            // `StdAck::Error`, so a counterparty cannot forge a timeout outcome
            // through an acknowledgement. The two recovery paths stay distinct.
            dispatch_profit_callback(
                deps.storage,
                envelope,
                RemoteOperationOutcome::OperationTimeout,
            )
        })
}

// Total by construction so the ack always commits: an over-cap `StdAck::Error`
// string is truncated to the byte cap on a char boundary, and an undecodable
// `StdAck::Success` payload becomes a synthetic `OperationErr`. The success
// payload is decoded as the wire shape only — currency-registry validation
// belongs to the addressee profit, which absorbs content failures. Erring here
// on counterparty content would make the relayer retry the ack forever,
// turning content drift into a stuck packet.
//
// SECURITY (FM5): this function maps ONLY the two `StdAck` variants a
// counterparty can author — `Success` and `Error` — into `OperationOk` or
// `OperationErr`. It can never yield `RemoteOperationOutcome::OperationTimeout`:
// a timeout is an IBC-layer fact the local chain observes, not a value the
// counterparty supplies, so it is constructed exclusively in
// `ibc_packet_timeout`. A counterparty-supplied acknowledgement therefore
// cannot drive the profit instance down the timeout recovery path (where funds
// may still be in flight).
fn ack_to_outcome(ack: StdAck) -> RemoteOperationOutcome {
    match ack {
        StdAck::Success(data) => cosmwasm_std::from_json::<WireOperationResponse>(&data)
            .map(RemoteOperationOutcome::OperationOk)
            .unwrap_or_else(|_err| {
                RemoteOperationOutcome::OperationErr(RemoteErrorMessage::truncated(
                    UNDECODABLE_ACK_REASON,
                ))
            }),
        StdAck::Error(message) => {
            RemoteOperationOutcome::OperationErr(RemoteErrorMessage::truncated(message))
        }
    }
}

// Callback routing for the SINGLETON remote profit. Unlike the multi-instance
// remote lease — whose addressee rides each packet envelope and is read back
// from the controller's own committed `original_packet.data` — the remote
// profit has exactly one local instance per port/channel (ADR-0008). Its
// address is fixed at instantiation in `Config.profit_contract`, so the
// callback target is loaded from storage rather than parsed off the wire. The
// `nonce`, however, is still read from the controller's OWN outbound envelope
// (never the counterparty's reply), so the profit can correlate the outcome to
// the exact in-flight emission.
fn dispatch_profit_callback(
    storage: &dyn Storage,
    envelope: PacketEnvelope,
    outcome: RemoteOperationOutcome,
) -> Result<IbcBasicResponse> {
    let callback = RemoteProfitCallback {
        nonce: envelope.nonce,
        outcome,
    };
    Config::load(storage)
        .map(|config| config.profit_contract().clone())
        .and_then(|profit_addr| {
            cosmwasm_std::to_json_binary(&ProfitExecuteMsg::RemoteProfitCallback(callback))
                .map_err(Error::from)
                .map(|msg| WasmMsg::Execute {
                    contract_addr: profit_addr.into_string(),
                    msg,
                    funds: vec![],
                })
        })
        .map(|wasm_msg| IbcBasicResponse::new().add_message(wasm_msg))
}

fn validate_handshake_channel(channel: &IbcChannel, config: &Config) -> Result<()> {
    require_unordered(channel.order.clone())
        .and_then(|()| require_version(&channel.version, &handshake_version(config)))
        .and_then(|()| require_connection_id(&channel.connection_id, config.connection_id()))
        .and_then(|()| {
            require_counterparty_port(&channel.counterparty_endpoint.port_id, config.dex_label())
        })
}

/// Validate the version the counterparty echoed in `OpenAck`. Both sides name
/// the same Solana-side transfer channel, so the echo must equal the proposed
/// handshake version verbatim. The counterparty-authored string lands in the
/// error truncated — never echoed unbounded.
fn validate_counterparty_version(may_actual: Option<&str>, config: &Config) -> Result<()> {
    may_actual.map_or(Ok(()), |actual| {
        let expected = handshake_version(config);
        if actual == expected {
            Ok(())
        } else {
            Err(Error::InvalidCounterpartyVersion {
                expected,
                actual: actual
                    .chars()
                    .take(COUNTERPARTY_VERSION_ECHO_MAX_CHARS)
                    .collect(),
            })
        }
    })
}

fn require_unordered(order: IbcOrder) -> Result<()> {
    match order {
        IbcOrder::Unordered => Ok(()),
        IbcOrder::Ordered => Err(Error::InvalidChannelOrdering),
    }
}

fn require_version(actual: &str, expected: &str) -> Result<()> {
    if actual == expected {
        Ok(())
    } else {
        Err(Error::InvalidChannelVersion {
            expected: expected.to_string(),
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
    let expected = remote_profit::port_id_for(dex_label);
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
