use std::ops::{Deref, DerefMut};

use access_control::SingleUserAccess;
use currencies::Nls;
use platform::{
    bank::{self, BankAccount, BankAccountView},
    error as platform_error,
    message::Response as PlatformResponse,
    response,
};
use sdk::{
    cosmwasm_ext::Response as CwResponse,
    cosmwasm_std::{
        self, Addr, Binary, Deps, DepsMut, Env, MessageInfo, QuerierWrapper, Storage, entry_point,
    },
};
use versioning::{
    ProtocolMigrationMessage, ProtocolPackageRelease, UpdatablePackage as _, VersionSegment,
    package_name, package_version,
};

use crate::{
    api::{ConfigResponse, ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg},
    error::{Error, Result},
    state::Config,
};

#[cfg(test)]
mod tests;

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
    new_vault: InstantiateMsg,
) -> Result<CwResponse> {
    deps.api
        .addr_validate(new_vault.owner.as_str())
        .map_err(Error::from)
        .and_then(|owner| {
            SingleUserAccess::new(deps.storage.deref_mut(), crate::access_control::OWNER_KEY)
                .grant_to(&owner)
                .map_err(Into::into)
                .and_then(|()| Config::new(owner).store(deps.storage))
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
    match msg {
        ExecuteMsg::Sweep { recipient } => authorize_owner_only(deps.storage.deref(), &info)
            .and_then(|()| sweep(&env.contract.address, recipient, deps.querier)),
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
        QueryMsg::ProtocolPackageRelease {} => {
            cosmwasm_std::to_json_binary(&CURRENT_RELEASE).map_err(Into::into)
        }
    }
    .inspect_err(platform_error::log(deps.api))
}

fn authorize_owner_only(store: &dyn Storage, call_message: &MessageInfo) -> Result<()> {
    SingleUserAccess::new(store, crate::access_control::OWNER_KEY)
        .check(call_message)
        .map_err(Into::into)
}

fn sweep(vault: &Addr, recipient: Addr, querier: QuerierWrapper<'_>) -> Result<PlatformResponse> {
    let mut vault_account = bank::account(vault, querier);
    vault_account
        .balance::<Nls>()
        .map_err(Error::ObtainBalance)
        .map(|balance| {
            if !balance.is_zero() {
                vault_account.send(balance, recipient);
            }
            PlatformResponse::messages_only(vault_account.into())
        })
}
