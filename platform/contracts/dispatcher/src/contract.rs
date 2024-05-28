use std::ops::DerefMut;

use access_control::SingleUserAccess;
use platform::response;
use sdk::{
    cosmwasm_ext::Response as CwResponse,
    cosmwasm_std::{entry_point, Binary, Deps, DepsMut, Env, MessageInfo, Storage},
};
use versioning::{package_version, version, FullUpdateOutput, SemVer, Version, VersionSegment};

use crate::{
    msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg, SudoMsg},
    result::ContractResult,
    state,
};

const CONTRACT_STORAGE_VERSION_FROM: VersionSegment = 1;
const CONTRACT_STORAGE_VERSION: VersionSegment = 2;
const PACKAGE_VERSION: SemVer = package_version!();
const CONTRACT_VERSION: Version = version!(CONTRACT_STORAGE_VERSION, PACKAGE_VERSION);

#[entry_point]
pub fn instantiate(
    _deps: DepsMut<'_>,
    _env: Env,
    _info: MessageInfo,
    _msg: InstantiateMsg,
) -> ContractResult<CwResponse> {
    unsupported()
}

#[entry_point]
pub fn migrate(
    deps: DepsMut<'_>,
    _env: Env,
    MigrateMsg {}: MigrateMsg,
) -> ContractResult<CwResponse> {
    versioning::update_software_and_storage::<CONTRACT_STORAGE_VERSION_FROM, _, _, _, _>(
        deps.storage,
        CONTRACT_VERSION,
        self::wipe_out,
        Into::into,
    )
    .and_then(
        |FullUpdateOutput {
             release_label,
             storage_migration_output: (),
         }| response::response(release_label),
    )
}

#[entry_point]
pub fn execute(
    _deps: DepsMut<'_>,
    _env: Env,
    _info: MessageInfo,
    msg: ExecuteMsg,
) -> ContractResult<CwResponse> {
    match msg {
        ExecuteMsg::TimeAlarm {} => Ok(response::empty_response()), // we just consume the time alarm
    }
}

#[entry_point]
pub fn sudo(_deps: DepsMut<'_>, _env: Env, _msg: SudoMsg) -> ContractResult<CwResponse> {
    unsupported()
}

#[entry_point]
pub fn query(_deps: Deps<'_>, _env: Env, _msg: QueryMsg) -> ContractResult<Binary> {
    unsupported()
}

fn unsupported() -> ! {
    unimplemented!(
        "Deprecated contract!!! The rewards dispatching has been moved to the Treasury contract"
    )
}

fn wipe_out(mut storage: &mut dyn Storage) -> ContractResult<()> {
    SingleUserAccess::new(
        storage.deref_mut(),
        crate::access_control::TIMEALARMS_NAMESPACE,
    )
    .revoke();
    state::wipe_out(storage);
    Ok(())
}
