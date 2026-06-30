use std::ops::{Deref, DerefMut};

use access_control::SingleUserAccess;
use cosmwasm_std::Storage;
use cw_time::{IntoInstant as _, IntoTimestamp as _};
use finance::{duration::Duration, instant::Instant};
use platform::{
    batch::Batch,
    contract::{self, Code, CodeId},
    error as platform_error,
    message::Response as PlatformResponse,
    response,
};
use remote_profit::{envelope::PacketEnvelope, msg::Operation, version::ProtocolVersion};
use sdk::{
    cosmwasm_ext::{CosmosMsg, Response as CwResponse},
    cosmwasm_std::{
        self, Addr, Api, Binary, Deps, DepsMut, Env, IbcMsg, IbcTimeout, MessageInfo,
        QuerierWrapper, entry_point,
    },
};
use versioning::{
    ProtocolMigrationMessage, ProtocolPackageRelease, UpdatablePackage as _, VersionSegment,
    package_name, package_version,
};

use crate::{
    api::{ChannelResponse, ConfigResponse, ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg},
    error::{Error, Result},
    ibc as ibc_msg, state,
    state::{Channel, Config},
};

const CONTRACT_STORAGE_VERSION: VersionSegment = 0;

/// Envelope nonce for the single-packet operations that have no duplicate-
/// callback window to close.
///
/// `OpenProfit`/`CloseProfit` solicit exactly one callback, so a superseded-
/// packet race cannot arise and they ride a zero nonce. `Swap` (#636) and the
/// multi-leg `TransferOut` drain (#671) instead carry a real per-emission nonce
/// the profit assigns, so a packet superseded by a re-emission or heal is
/// rejected.
const NO_NONCE: u64 = 0;
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
            grant_protocol_admin(
                deps.storage.deref_mut(),
                deps.api,
                new_controller.protocol_admin.as_str(),
            )
        })
        .and_then(|()| build_config(deps.api, deps.querier, new_controller))
        .and_then(|config| config.store(deps.storage))
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
        .map_err(Error::UpdateSoftware)
        .and_then(|()| Config::require_current_schema(deps.storage))
        .map(|()| response::empty_response())
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
        ExecuteMsg::NewProfitCode {
            profit_code: new_profit_code,
        } => authorize_protocol_admin_only(deps.storage.deref(), &info)
            .and_then(|()| set_profit_code(deps, new_profit_code))
            .map(|()| PlatformResponse::default()),
        ExecuteMsg::OpenProfit { params, timeout } => send_operation(
            deps,
            &env,
            info.sender,
            Operation::OpenProfit(params),
            timeout,
            NO_NONCE,
        ),
        ExecuteMsg::CloseProfit { params, timeout } => send_operation(
            deps,
            &env,
            info.sender,
            Operation::CloseProfit(params),
            timeout,
            NO_NONCE,
        ),
        ExecuteMsg::Swap {
            params,
            timeout,
            nonce,
        } => send_operation(
            deps,
            &env,
            info.sender,
            Operation::Swap(params),
            timeout,
            nonce,
        ),
        ExecuteMsg::TransferOut {
            params,
            timeout,
            nonce,
        } => send_operation(
            deps,
            &env,
            info.sender,
            Operation::TransferOut(params),
            timeout,
            nonce,
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

/// Reject a non-canonical channel id here, at instantiation, instead of
/// letting the handshake fail cross-chain at the counterparty's responder.
fn require_canonical_transfer_channel(channel_id: &str) -> Result<()> {
    if state::canonical_transfer_channel(channel_id) {
        Ok(())
    } else {
        Err(Error::NonCanonicalTransferChannel(channel_id.to_string()))
    }
}

fn authorize_protocol_admin_only(store: &dyn Storage, call_message: &MessageInfo) -> Result<()> {
    SingleUserAccess::new(store, crate::access_control::PROTOCOL_ADMIN_KEY)
        .check(call_message)
        .map_err(Into::into)
}

fn grant_protocol_admin(
    storage: &mut dyn Storage,
    api: &dyn Api,
    protocol_admin: &str,
) -> Result<()> {
    api.addr_validate(protocol_admin)
        .map_err(Error::from)
        // cannot validate the protocol admin contract for existence, since it is not yet instantiated
        .and_then(|admin| {
            SingleUserAccess::new(storage, crate::access_control::PROTOCOL_ADMIN_KEY)
                .grant_to(&admin)
                .map_err(Into::into)
        })
}

/// Validate the profit identity inputs and assemble the stored [`Config`].
fn build_config(
    api: &dyn Api,
    querier: QuerierWrapper<'_>,
    new_controller: InstantiateMsg,
) -> Result<Config> {
    // The profit contract is a deploy-supplied trusted input: its address is
    // bech32-validated but NOT checked to be an instance of `profit_code`. The
    // coordinated cutover may instantiate this controller before the profit
    // instance exists (as with the protocol admin), so a misconfigured target is
    // surfaced by the first callback's `WasmMsg::Execute` rather than here.
    api.addr_validate(new_controller.profit_contract.as_str())
        .map_err(Error::from)
        .and_then(|profit_contract| {
            new_controller
                .profit_code
                .try_validate(&contract::validator(querier))
                .map_err(Error::from)
                .map(|profit_code| {
                    Config::new(
                        new_controller.connection_id,
                        new_controller.dex_label,
                        new_controller.transfer_channel,
                        profit_code,
                        profit_contract,
                    )
                })
        })
}

/// Confirm the rotated profit code exists on-chain before persisting it.
///
/// `instantiate` validates `profit_code` through `try_validate`; this mirrors
/// that check on the update path so an admin cannot persist a non-existent code
/// id that would then reject every authorised caller and brick outbound emission.
fn set_profit_code(deps: DepsMut<'_>, new_profit_code: Code) -> Result<()> {
    Code::try_new(
        CodeId::from(new_profit_code),
        &contract::validator(deps.querier),
    )
    .map_err(Error::from)
    .and_then(|validated_code| Config::update_profit_code(deps.storage, validated_code))
}

fn open_channel(storage: &mut dyn Storage, env: &Env) -> Result<PlatformResponse> {
    Channel::may_load(storage)
        .and_then(|existing| match existing {
            Some(_) => Err(Error::ChannelAlreadyExists),
            None => Config::load(storage),
        })
        .map(|config| {
            let open_init: CosmosMsg = ibc_msg::build_channel_open_init(env, &config);
            let mut batch = Batch::default();
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
                let mut batch = Batch::default();
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
    nonce: u64,
) -> Result<PlatformResponse> {
    let DepsMut {
        storage, querier, ..
    } = deps;
    authorise_and_load_channel(storage, querier, caller).and_then(|channel| {
        build_packet(
            env.block.time.into_instant(),
            &channel,
            operation,
            timeout,
            nonce,
        )
    })
}

fn authorise_and_load_channel(
    storage: &dyn Storage,
    querier: QuerierWrapper<'_>,
    caller: Addr,
) -> Result<Channel> {
    Config::load(storage)
        // The remote profit is a singleton, so the authorised caller's address
        // is NOT placed on the packet envelope (the way the multi-instance lease
        // carries its addressee). `auth_caller` is retained purely as the
        // code-id sender gate — only an instance of `Config.profit_code` may
        // emit a packet; the returned address is intentionally discarded.
        .and_then(|config| {
            config
                .auth_caller(querier, caller)
                .map(drop)
                .map(|()| config)
        })
        .and_then(|_config| {
            Channel::may_load(storage)
                .and_then(|maybe| maybe.ok_or(Error::ChannelNotOpen))
                .and_then(|channel| channel.usable_or_err().map(|()| channel))
        })
}

fn build_packet(
    now: Instant,
    channel: &Channel,
    operation: Operation,
    timeout: Duration,
    nonce: u64,
) -> Result<PlatformResponse> {
    let envelope = PacketEnvelope {
        operation,
        version: ProtocolVersion,
        nonce,
    };
    cosmwasm_std::to_json_binary(&envelope)
        .map_err(Error::from)
        .map(|data| {
            let send = CosmosMsg::Ibc(IbcMsg::SendPacket {
                channel_id: channel.local_channel_id().to_string(),
                data,
                timeout: IbcTimeout::with_timestamp((now + timeout).into_timestamp()),
            });
            let mut batch = Batch::default();
            batch.schedule_execute_no_reply(send);
            PlatformResponse::messages_only(batch)
        })
}

#[cfg(test)]
mod tests;
