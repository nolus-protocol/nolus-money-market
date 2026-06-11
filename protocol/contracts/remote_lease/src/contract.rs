use std::ops::{Deref, DerefMut};

use access_control::SingleUserAccess;
use cosmwasm_std::Storage;
use cw_time::{IntoInstant as _, IntoTimestamp as _};
use finance::{duration::Duration, instant::Instant};
use platform::{
    contract::Code, error as platform_error, message::Response as PlatformResponse, response,
};
use remote_lease::{
    envelope::{LeaseAddrOnWire, PacketEnvelope},
    msg::Operation,
    version::ProtocolVersion,
};
use sdk::{
    cosmwasm_ext::{CosmosMsg, Response as CwResponse},
    cosmwasm_std::{
        self, Addr, Binary, Deps, DepsMut, Env, IbcMsg, IbcTimeout, MessageInfo, QuerierWrapper,
        entry_point,
    },
};
use versioning::{
    ProtocolMigrationMessage, ProtocolPackageRelease, UpdatablePackage as _, VersionSegment,
    package_name, package_version,
};

use crate::{
    api::{ChannelResponse, ConfigResponse, ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg},
    error::{Error, Result},
    ibc as ibc_msg,
    state::{Channel, Config},
};

const CONTRACT_STORAGE_VERSION: VersionSegment = 0;
const CURRENT_RELEASE: ProtocolPackageRelease = ProtocolPackageRelease::current(
    package_name!(),
    package_version!(),
    CONTRACT_STORAGE_VERSION,
);

#[entry_point]
pub fn instantiate(
    mut deps: DepsMut<'_>,
    _env: Env,
    _info: MessageInfo,
    new_controller: InstantiateMsg,
) -> Result<CwResponse> {
    require_non_empty("connection_id", &new_controller.connection_id)
        .and_then(|()| require_non_empty("dex_label", &new_controller.dex_label))
        .and_then(|()| require_canonical_transfer_channel(&new_controller.transfer_channel))
        .and_then(|()| {
            deps.api
                .addr_validate(new_controller.protocol_admin.as_str())
                .map_err(Error::from)
        })
        // cannot validate the protocol admin contract for existence, since it is not yet instantiated
        .and_then(|protocol_admin| {
            SingleUserAccess::new(
                deps.storage.deref_mut(),
                crate::access_control::PROTOCOL_ADMIN_KEY,
            )
            .grant_to(&protocol_admin)
            .map_err(Into::into)
        })
        .and_then(|()| {
            Code::try_new(
                new_controller.lease_code.into(),
                &platform::contract::validator(deps.querier),
            )
            .map_err(Into::into)
        })
        .and_then(|lease_code| {
            Config::new(
                new_controller.connection_id,
                new_controller.dex_label,
                new_controller.transfer_channel,
                lease_code,
            )
            .store(deps.storage)
        })
        .map(|()| response::empty_response())
        .inspect_err(platform_error::log(deps.api))
}

#[entry_point]
pub fn migrate(
    deps: DepsMut<'_>,
    _env: Env,
    ProtocolMigrationMessage {
        migrate_from,
        to_release,
        message: MigrateMsg {},
    }: ProtocolMigrationMessage<MigrateMsg>,
) -> Result<CwResponse> {
    migrate_from
        .update_software(&CURRENT_RELEASE, &to_release)
        .map(|()| response::empty_response())
        .map_err(Error::UpdateSoftware)
        .inspect_err(platform_error::log(deps.api))
}

#[entry_point]
pub fn execute(
    deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<CwResponse> {
    let api = deps.api;
    match msg {
        ExecuteMsg::OpenChannel() => authorize_protocol_admin_only(deps.storage.deref(), &info)
            .and_then(|()| open_channel(deps.storage, &env)),
        ExecuteMsg::CloseChannel() => authorize_protocol_admin_only(deps.storage.deref(), &info)
            .and_then(|()| close_channel(deps.storage)),
        ExecuteMsg::NewLeaseCode {
            lease_code: new_lease_code,
        } => authorize_protocol_admin_only(deps.storage.deref(), &info)
            .and_then(|()| Config::update_lease_code(deps.storage, new_lease_code))
            .map(|()| PlatformResponse::default()),
        ExecuteMsg::OpenLease { params, timeout } => send_operation(
            deps,
            &env,
            info.sender,
            Operation::OpenLease(params),
            timeout,
        ),
        ExecuteMsg::CloseLease { params, timeout } => send_operation(
            deps,
            &env,
            info.sender,
            Operation::CloseLease(params),
            timeout,
        ),
        ExecuteMsg::Swap { params, timeout } => {
            send_operation(deps, &env, info.sender, Operation::Swap(params), timeout)
        }
        ExecuteMsg::TransferOut { params, timeout } => send_operation(
            deps,
            &env,
            info.sender,
            Operation::TransferOut(params),
            timeout,
        ),
    }
    .map(response::response_only_messages)
    .inspect_err(platform_error::log(api))
}

#[entry_point]
pub fn query(deps: Deps<'_>, _env: Env, msg: QueryMsg) -> Result<Binary> {
    match msg {
        QueryMsg::Config() => Config::load(deps.storage)
            .map(ConfigResponse::from)
            .and_then(|config| cosmwasm_std::to_json_binary(&config).map_err(Into::into)),
        QueryMsg::Channel() => Channel::may_load(deps.storage)
            .map(ChannelResponse::from)
            .and_then(|channel| cosmwasm_std::to_json_binary(&channel).map_err(Into::into)),
        QueryMsg::ProtocolPackageRelease {} => {
            cosmwasm_std::to_json_binary(&CURRENT_RELEASE).map_err(Into::into)
        }
    }
    .inspect_err(platform_error::log(deps.api))
}

fn require_non_empty(field: &'static str, value: &str) -> Result<()> {
    if value.is_empty() {
        Err(Error::EmptyInstantiateField(field))
    } else {
        Ok(())
    }
}

const TRANSFER_CHANNEL_NAME_PREFIX: &str = "channel-";

/// Accept only the canonical decimal rendering of a `u16` ordinal — the
/// counterparty's responder rejects leading zeros, signs, and ordinals beyond
/// its 16-bit entity range, so a non-canonical id would fail the handshake
/// cross-chain instead of failing here at instantiation.
fn require_canonical_transfer_channel(channel_id: &str) -> Result<()> {
    channel_id
        .strip_prefix(TRANSFER_CHANNEL_NAME_PREFIX)
        .and_then(|ordinal| {
            ordinal
                .parse::<u16>()
                .ok()
                .filter(|parsed| parsed.to_string() == ordinal)
        })
        .map(|_ordinal| ())
        .ok_or_else(|| Error::NonCanonicalTransferChannel(channel_id.to_string()))
}

fn authorize_protocol_admin_only(store: &dyn Storage, call_message: &MessageInfo) -> Result<()> {
    SingleUserAccess::new(store, crate::access_control::PROTOCOL_ADMIN_KEY)
        .check(call_message)
        .map_err(Into::into)
}

fn open_channel(storage: &mut dyn Storage, env: &Env) -> Result<PlatformResponse> {
    Channel::may_load(storage)
        .and_then(|existing| match existing {
            Some(_) => Err(Error::ChannelAlreadyExists),
            None => Config::load(storage),
        })
        .map(|config| {
            let open_init: CosmosMsg = ibc_msg::build_channel_open_init(env, &config);
            let mut batch = platform::batch::Batch::default();
            batch.schedule_execute_no_reply(open_init);
            PlatformResponse::messages_only(batch)
        })
}

fn close_channel(storage: &mut dyn Storage) -> Result<PlatformResponse> {
    Channel::may_load(storage)
        .and_then(|maybe_channel| maybe_channel.ok_or(Error::ChannelNotOpen))
        .and_then(Channel::into_closing)
        .and_then(|closing| {
            closing.store(storage).map(|()| {
                let close_msg: CosmosMsg = ibc_msg::build_channel_close(&closing);
                let mut batch = platform::batch::Batch::default();
                batch.schedule_execute_no_reply(close_msg);
                PlatformResponse::messages_only(batch)
            })
        })
}

fn send_operation(
    deps: DepsMut<'_>,
    env: &Env,
    caller: Addr,
    operation: Operation,
    timeout: Duration,
) -> Result<PlatformResponse> {
    let DepsMut {
        storage, querier, ..
    } = deps;
    authorise_and_load_channel(storage, querier, caller).and_then(|(lease, channel)| {
        build_packet(
            env.block.time.into_instant(),
            &channel,
            lease,
            operation,
            timeout,
        )
    })
}

fn authorise_and_load_channel(
    storage: &dyn Storage,
    querier: QuerierWrapper<'_>,
    caller: Addr,
) -> Result<(Addr, Channel)> {
    Config::load(storage)
        .and_then(|config| config.auth_caller(querier, caller))
        .and_then(|lease| {
            Channel::may_load(storage)
                .and_then(|maybe| maybe.ok_or(Error::ChannelNotOpen))
                .and_then(|channel| channel.usable_or_err().map(|()| (lease, channel)))
        })
}

fn build_packet(
    now: Instant,
    channel: &Channel,
    lease: Addr,
    operation: Operation,
    timeout: Duration,
) -> Result<PlatformResponse> {
    let envelope = PacketEnvelope {
        lease: LeaseAddrOnWire::new(lease),
        operation,
        version: ProtocolVersion,
    };
    cosmwasm_std::to_json_binary(&envelope)
        .map_err(Error::from)
        .map(|data| {
            let send = CosmosMsg::Ibc(IbcMsg::SendPacket {
                channel_id: channel.local_channel_id().to_string(),
                data,
                timeout: IbcTimeout::with_timestamp((now + timeout).into_timestamp()),
            });
            let mut batch = platform::batch::Batch::default();
            batch.schedule_execute_no_reply(send);
            PlatformResponse::messages_only(batch)
        })
}

#[cfg(test)]
mod tests;
