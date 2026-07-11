use std::ops::Deref;

use access_control::{ContractOwnerAccess, SingleUserAccess};
use cw_time::IntoInstant as _;
use platform::{
    batch::Batch, contract, error as platform_error, message::Response as MessageResponse, response,
};
use sdk::{
    cosmwasm_ext::Response as CwResponse,
    cosmwasm_std::{
        Api, Binary, Deps, DepsMut, Env, MessageInfo, QuerierWrapper, Storage, entry_point,
    },
};
use timealarms::stub::TimeAlarmsRef;
use versioning::{
    ProtocolMigrationMessage, ProtocolPackageRelease, UpdatablePackage as _, VersionSegment,
    package_name, package_version,
};

use crate::{
    CadenceHours,
    error::ContractError,
    msg::{ConfigResponse, ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg},
    profit,
    result::ContractResult,
    state::Config,
};

const CONTRACT_STORAGE_VERSION: VersionSegment = 2;
const CURRENT_RELEASE: ProtocolPackageRelease = ProtocolPackageRelease::current(
    package_name!(),
    package_version!(),
    CONTRACT_STORAGE_VERSION,
);

#[entry_point]
pub fn instantiate(
    deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> ContractResult<CwResponse> {
    setup(deps.storage, deps.querier, deps.api, &env, info, msg)
        .map(response::response_only_messages)
        .inspect_err(platform_error::log(deps.api))
}

/// Fresh-deploy only. The new [`Config`] reuses the `"contract_state"` storage
/// key with an incompatible layout, so `CONTRACT_STORAGE_VERSION` is bumped to 2:
/// `update_software` refuses to migrate a pre-settlement (storage v1) instance on
/// the storage-version mismatch rather than letting `Config::load` read the old
/// `State` bytes. Profit is redeployed as a fresh instance; there is no live
/// state to migrate.
#[entry_point]
pub fn migrate(
    deps: DepsMut<'_>,
    _env: Env,
    ProtocolMigrationMessage {
        migrate_from,
        to_release,
        message: MigrateMsg {},
    }: ProtocolMigrationMessage<MigrateMsg>,
) -> ContractResult<CwResponse> {
    migrate_from
        .update_software(&CURRENT_RELEASE, &to_release)
        .map(|()| response::empty_response())
        .map_err(ContractError::UpdateSoftware)
        .inspect_err(platform_error::log(deps.api))
}

#[entry_point]
pub fn execute(
    deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> ContractResult<CwResponse> {
    match msg {
        ExecuteMsg::TimeAlarm {} => {
            SingleUserAccess::new(
                deps.storage.deref(),
                crate::access_control::TIMEALARMS_NAMESPACE,
            )
            .check(&info)?;

            Config::load(deps.storage)
                .and_then(|config: Config| profit::on_time_alarm(&config, &env, deps.querier))
                .map(response::response_only_messages)
        }
        ExecuteMsg::Config { cadence_hours } => {
            ContractOwnerAccess::new(deps.storage.deref()).check(&info)?;

            update_cadence(deps.storage, &env, cadence_hours).map(response::response_only_messages)
        }
    }
    .inspect_err(platform_error::log(deps.api))
}

#[entry_point]
pub fn query(deps: Deps<'_>, _env: Env, msg: QueryMsg) -> ContractResult<Binary> {
    match msg {
        QueryMsg::Config {} => Config::load(deps.storage)
            .map(|config: Config| ConfigResponse {
                cadence_hours: config.cadence_hours(),
            })
            .and_then(|resp: ConfigResponse| {
                cosmwasm_std::to_json_binary(&resp).map_err(Into::into)
            }),
        QueryMsg::ProtocolPackageRelease {} => {
            cosmwasm_std::to_json_binary(&CURRENT_RELEASE).map_err(Into::into)
        }
    }
}

fn setup(
    storage: &mut dyn Storage,
    querier: QuerierWrapper<'_>,
    api: &dyn Api,
    env: &Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> ContractResult<MessageResponse> {
    api.addr_validate(msg.settlement.as_str())?;

    ContractOwnerAccess::new(&mut *storage).grant_to(&info)?;
    SingleUserAccess::new(&mut *storage, crate::access_control::TIMEALARMS_NAMESPACE)
        .grant_to(&msg.timealarms)?;

    TimeAlarmsRef::new(msg.timealarms, &contract::validator(querier))
        .map_err(ContractError::from)
        .map(|time_alarms: TimeAlarmsRef| {
            Config::new(msg.cadence_hours, msg.settlement, time_alarms)
        })
        .and_then(|config: Config| store_and_arm(storage, env, config))
}

fn update_cadence(
    storage: &mut dyn Storage,
    env: &Env,
    cadence_hours: CadenceHours,
) -> ContractResult<MessageResponse> {
    Config::load(storage)
        .map(|config: Config| config.update_cadence_hours(cadence_hours))
        .and_then(|config: Config| store_and_arm(storage, env, config))
}

fn store_and_arm(
    storage: &mut dyn Storage,
    env: &Env,
    config: Config,
) -> ContractResult<MessageResponse> {
    profit::setup_alarm(&config, env.block.time.into_instant()).and_then(|alarm: Batch| {
        config
            .store(storage)
            .map(|()| MessageResponse::messages_only(alarm))
    })
}
