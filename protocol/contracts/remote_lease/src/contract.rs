use std::ops::{Deref, DerefMut};

use access_control::SingleUserAccess;
use cosmwasm_std::Storage;
use platform::{
    contract::Code, error as platform_error, message::Response as PlatformResponse, response,
};
use sdk::{
    cosmwasm_ext::Response as CwResponse,
    cosmwasm_std::{self, Binary, Deps, DepsMut, Env, MessageInfo, entry_point},
};
use versioning::{
    ProtocolMigrationMessage, ProtocolPackageRelease, UpdatablePackage as _, VersionSegment,
    package_name, package_version,
};

use crate::{
    api::{ChannelResponse, ConfigResponse, ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg},
    error::{Error, Result},
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
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<CwResponse> {
    match msg {
        ExecuteMsg::NewLeaseCode(code) => {
            authorize_protocol_admin_only(deps.storage.deref(), &info)
                .and_then(|()| Config::update_lease_code(deps.storage, code))
                .map(|()| PlatformResponse::default())
        }
    }
    .map(response::response_only_messages)
    .inspect_err(platform_error::log(deps.api))
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

fn authorize_protocol_admin_only(store: &dyn Storage, call_message: &MessageInfo) -> Result<()> {
    SingleUserAccess::new(store, crate::access_control::PROTOCOL_ADMIN_KEY)
        .check(call_message)
        .map_err(Into::into)
}

#[cfg(test)]
mod tests;
