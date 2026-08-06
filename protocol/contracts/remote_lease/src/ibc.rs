use currencies::{LeaseGroup, Lpns as LpnGroup, PaymentGroup};
use remote_lease::{
    callback::{RemoteError, RemoteLeaseCallback},
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
        IbcReceiveResponse, Never, StdAck, Storage, WasmMsg, entry_point,
    },
    ibc_proto::ibc::core::channel::v1::{
        Channel as ProtoChannel, Counterparty as ProtoCounterparty, MsgChannelOpenInit,
        Order as ProtoOrder, State as ProtoState,
    },
};

use crate::{
    error::{Error, Result},
    lease_callback::LeaseExecuteMsg,
    state::{Channel, Config},
};

const MSG_CHANNEL_OPEN_INIT_TYPE_URL: &str = "/ibc.core.channel.v1.MsgChannelOpenInit";

/// Build the `CosmosMsg::Any { MsgChannelOpenInit }` that initiates the
/// handshake, proposing `channel_version` to the counterparty.
pub fn build_channel_open_init(env: &Env, config: &Config, channel_version: &str) -> CosmosMsg {
    let counterparty_port_id = remote_lease::port_id_for(config.dex_label());
    let channel = ProtoChannel {
        state: ProtoState::Init.into(),
        ordering: ProtoOrder::Unordered.into(),
        counterparty: Some(ProtoCounterparty {
            port_id: counterparty_port_id,
            channel_id: String::new(),
        }),
        connection_hops: vec![config.connection_id().to_string()],
        version: channel_version.to_string(),
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
pub fn build_channel_close(local_channel_id: &str) -> CosmosMsg {
    CosmosMsg::Ibc(IbcMsg::CloseChannel {
        channel_id: local_channel_id.to_string(),
    })
}

#[entry_point]
pub fn ibc_channel_open(
    deps: DepsMut<'_>,
    _env: Env,
    msg: IbcChannelOpenMsg,
) -> Result<IbcChannelOpenResponse> {
    match msg {
        // Consuming the proposal here is what makes the callback single-use:
        // wasmd dispatches our `MsgChannelOpenInit` in the same transaction
        // that emitted it, so a second `OpenInit` finds nothing to consume.
        IbcChannelOpenMsg::OpenInit { channel } => Config::load(deps.storage)
            .and_then(|config| load_channel(deps.storage).map(|recorded| (config, recorded)))
            .and_then(|(config, recorded)| {
                let expected = recorded.version();
                validate_handshake_channel(&channel, &config, &expected)
                    .and_then(|()| recorded.into_init_accepted(channel.endpoint.channel_id))
            })
            .and_then(|accepted| accepted.store(deps.storage))
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
    match msg {
        IbcChannelConnectMsg::OpenAck {
            channel,
            counterparty_version,
        } => Config::load(deps.storage)
            .and_then(|config| load_channel(deps.storage).map(|recorded| (config, recorded)))
            .and_then(|(config, recorded)| {
                let expected = recorded.version();
                validate_handshake_channel(&channel, &config, &expected)
                    .and_then(|()| require_counterparty_version(&counterparty_version, &expected))
                    .and_then(|()| establish(recorded, channel))
            })
            .and_then(|established| established.store(deps.storage))
            .map(|()| IbcBasicResponse::new()),
        // The callback of an `OpenTry` this contract rejects, and it carries no
        // counterparty version to prove the pairing — fail closed rather than
        // persist an unproven channel should an upstream change ever route it
        // here.
        IbcChannelConnectMsg::OpenConfirm { .. } => Err(Error::UnsupportedCounterpartyOpen),
    }
}

#[entry_point]
pub fn ibc_channel_close(
    deps: DepsMut<'_>,
    _env: Env,
    msg: IbcChannelCloseMsg,
) -> Result<IbcBasicResponse> {
    match msg {
        // The completion of a close this controller initiated: ibc-go closes
        // the local end within the `MsgChannelCloseInit` this callback belongs
        // to, so this is where the record is cleared. `CloseConfirm` runs on
        // the passive side only — never reached here, since the counterparty
        // rejects close-inits of its own — so it fails closed like
        // `OpenConfirm`.
        IbcChannelCloseMsg::CloseInit { .. } => Channel::may_load(deps.storage)
            .and_then(|maybe_channel| {
                maybe_channel.map_or(Err(Error::UnsolicitedChannelClose), |channel| {
                    channel.close_init_or_err()
                })
            })
            .map(|()| {
                Channel::clear(deps.storage);
                IbcBasicResponse::new()
            }),
        IbcChannelCloseMsg::CloseConfirm { .. } => Err(Error::UnsolicitedChannelClose),
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

// `StdAck::Error` is a bare string, so this is the single point where the
// counterparty's failure becomes typed: the kind is parsed once here, leaving
// consumers a value to branch on rather than prose to re-parse. Both fallible
// steps — `parse_ack`'s code frame and its length cap — reject counterparty
// non-conformance rather than guessing a kind. That does mean an `Err` here
// reverts `ibc_packet_ack` (the dispatch below is a plain `add_message`) and
// leaves the relayer redelivering, which is the intended loud failure: the two
// ends of this protocol deploy in lockstep, so an unparseable code is a
// deployment fault to fix, not a case to absorb. A *classified* failure never
// returns `Err` from here.
fn ack_to_callback(ack: StdAck) -> Result<RemoteLeaseCallback<PaymentGroup>> {
    match ack {
        StdAck::Success(data) => cosmwasm_std::from_json::<OperationResponse<PaymentGroup>>(&data)
            .map(RemoteLeaseCallback::OperationOk)
            .map_err(Error::from),
        StdAck::Error(message) => RemoteError::parse_ack(message)
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

fn load_channel(storage: &dyn Storage) -> Result<Channel> {
    Channel::may_load(storage)
        .and_then(|maybe_channel| maybe_channel.ok_or(Error::UnsolicitedChannelOpen))
}

fn establish(recorded: Channel, channel: IbcChannel) -> Result<Channel> {
    let IbcChannel {
        endpoint,
        counterparty_endpoint,
        ..
    } = channel;
    recorded.into_established(
        endpoint.channel_id,
        counterparty_endpoint.channel_id,
        counterparty_endpoint.port_id,
    )
}

fn validate_handshake_channel(
    channel: &IbcChannel,
    config: &Config,
    expected_version: &str,
) -> Result<()> {
    require_unordered(channel.order.clone())
        .and_then(|()| require_version(&channel.version, expected_version))
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

fn require_version(actual: &str, expected: &str) -> Result<()> {
    if actual == expected {
        Ok(())
    } else {
        Err(Error::InvalidChannelVersion {
            expected: expected.to_string(),
            actual: bounded(actual),
        })
    }
}

// The counterparty echoes the version it accepted verbatim, so an exact match
// against our own proposal is what proves it bound the transfer channel we
// asked for rather than one of its choosing.
fn require_counterparty_version(actual: &str, expected: &str) -> Result<()> {
    if actual == expected {
        Ok(())
    } else {
        Err(Error::InvalidCounterpartyVersion {
            expected: expected.to_string(),
            actual: bounded(actual),
        })
    }
}

fn bounded(version: &str) -> String {
    remote_lease::channel_version::bounded_channel_version(version).to_string()
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

fn contract_port_id(contract: &Addr) -> String {
    format!("wasm.{contract}")
}

#[cfg(test)]
mod tests;
