use platform::response;
#[cfg(feature = "contract-with-bindings")]
use sdk::cosmwasm_std::entry_point;
use sdk::{
    cosmwasm_ext::Response as CwResponse,
    cosmwasm_std::{ensure_eq, DepsMut, Env, MessageInfo, Reply, Storage},
};
use versioning::{package_version, version, ReleaseLabel, SemVer, Version, VersionSegment};

use self::{
    error::Error as ContractError,
    msg::{InstantiateMsg, MigrateMsg, SudoMsg},
    result::Result as ContractResult,
    state::{contracts as state_contracts, migration_release},
};

pub mod common;
pub mod error;
pub mod migrate_contracts;
pub mod msg;
pub mod result;
pub mod state;

// version info for migration info
const CONTRACT_STORAGE_VERSION_FROM: VersionSegment = 0;
const CONTRACT_STORAGE_VERSION: VersionSegment = 1;
const PACKAGE_VERSION: SemVer = package_version!();
const CONTRACT_VERSION: Version = version!(CONTRACT_STORAGE_VERSION, PACKAGE_VERSION);

#[cfg_attr(feature = "contract-with-bindings", entry_point)]
pub fn instantiate(
    deps: DepsMut<'_>,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> ContractResult<CwResponse> {
    versioning::initialize(deps.storage, CONTRACT_VERSION)?;

    msg.validate(&deps.querier)?;

    state_contracts::store(deps.storage, msg.contracts).map(|()| response::empty_response())
}

#[cfg_attr(feature = "contract-with-bindings", entry_point)]
pub fn migrate(
    deps: DepsMut<'_>,
    _env: Env,
    MigrateMsg { dex }: MigrateMsg,
) -> ContractResult<CwResponse> {
    versioning::update_software_and_storage::<CONTRACT_STORAGE_VERSION_FROM, _, _, _, _>(
        deps.storage,
        CONTRACT_VERSION,
        |storage: &mut dyn Storage| state_contracts::migrate(storage, dex),
        Into::into,
    )
    .and_then(|(label, ()): (ReleaseLabel, ())| response::response(label))
}

#[cfg_attr(feature = "contract-with-bindings", entry_point)]
pub fn sudo(deps: DepsMut<'_>, env: Env, msg: SudoMsg) -> ContractResult<CwResponse> {
    match msg {
        SudoMsg::AddProtocolSet { dex, ref contracts } => {
            state_contracts::add_dex_bound_set(deps.storage, dex, contracts)
                .map(|()| response::empty_response())
        }
        SudoMsg::MigrateContracts(migrate_contracts) => {
            migrate_contracts::migrate(deps.storage, env.contract.address, migrate_contracts)
                .map(response::response_only_messages)
        }
    }
}

#[cfg_attr(feature = "contract-with-bindings", entry_point)]
pub fn reply(deps: DepsMut<'_>, _env: Env, msg: Reply) -> ContractResult<CwResponse> {
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

    Ok(response::empty_response())
}
