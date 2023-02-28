#[cfg(feature = "contract-with-bindings")]
use sdk::cosmwasm_std::entry_point;
use sdk::{
    cosmwasm_ext::Response,
    cosmwasm_std::{ensure_eq, DepsMut, Env, MessageInfo, Reply},
};
use versioning::{respond_with_release, version, VersionSegment};

use self::{
    common::ValidateAddresses as _,
    error::ContractError,
    msg::{InstantiateMsg, MigrateMsg, SudoMsg},
    state::{contracts as state_contracts, migration_release},
};

pub mod common;
pub mod error;
pub mod migrate_contracts;
pub mod msg;
pub mod state;

// version info for migration info
// const CONTRACT_STORAGE_VERSION_FROM: VersionSegment = 0;
const CONTRACT_STORAGE_VERSION: VersionSegment = 0;

#[cfg_attr(feature = "contract-with-bindings", entry_point)]
pub fn instantiate(
    deps: DepsMut<'_>,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    versioning::initialize(deps.storage, version!(CONTRACT_STORAGE_VERSION))?;

    msg.general_contracts.validate(&deps.querier)?;
    msg.lpn_contracts.validate(&deps.querier)?;

    state_contracts::store(deps.storage, msg.general_contracts, msg.lpn_contracts)?;

    Ok(Response::default())
}

#[cfg_attr(feature = "contract-with-bindings", entry_point)]
pub fn migrate(deps: DepsMut<'_>, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    versioning::update_software(deps.storage, version!(CONTRACT_STORAGE_VERSION))?;

    respond_with_release().map_err(Into::into)
}

#[cfg_attr(feature = "contract-with-bindings", entry_point)]
pub fn sudo(deps: DepsMut<'_>, env: Env, msg: SudoMsg) -> Result<Response, ContractError> {
    match msg {
        SudoMsg::MigrateContracts(migrate_contracts) => {
            migrate_contracts::migrate(deps.storage, env.contract.address, migrate_contracts)
        }
    }
}

#[cfg_attr(feature = "contract-with-bindings", entry_point)]
pub fn reply(deps: DepsMut<'_>, _env: Env, msg: Reply) -> Result<Response, ContractError> {
    let expected_release: String = migration_release::load(deps.storage)?;

    let reported_release: String =
        platform::reply::from_execute(msg)?.ok_or(ContractError::NoMigrationResponseData {})?;

    ensure_eq!(
        reported_release,
        expected_release,
        ContractError::WrongRelease {
            reported: reported_release,
            expected: expected_release
        }
    );

    Ok(Response::default())
}
