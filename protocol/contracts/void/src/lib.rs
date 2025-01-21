use std::convert::Infallible;

use serde::{Deserialize, Serialize};

use platform::response;
use sdk::{
    cosmwasm_ext::Response as CwResponse,
    cosmwasm_std::{entry_point, Binary, Deps, DepsMut, Env, MessageInfo},
};
use timealarms::msg::ExecuteAlarmMsg;
use versioning::{self, package_name, package_version, PackageRelease, VersionSegment};

#[derive(Serialize, Deserialize)]
pub struct EmptyMsg {}

#[entry_point]
pub fn instantiate(
    _deps: DepsMut<'_>,
    _env: Env,
    _info: MessageInfo,
    EmptyMsg {}: EmptyMsg,
) -> Result<CwResponse, Infallible> {
    unimplemented!("Instantiation of a Void contract is not allowed!");
}

#[entry_point]
pub fn migrate(
    _deps: DepsMut<'_>,
    _env: Env,
    EmptyMsg {}: EmptyMsg,
) -> Result<CwResponse, platform::error::Error> {
    const CONTRACT_STORAGE_VERSION: VersionSegment = 1;
    const CURRENT_RELEASE: PackageRelease = PackageRelease::current(
        package_name!(),
        package_version!(),
        CONTRACT_STORAGE_VERSION,
    );

    response::response(CURRENT_RELEASE.release())
}

#[entry_point]
pub fn execute(
    _deps: DepsMut<'_>,
    _env: Env,
    _info: MessageInfo,
    msg: ExecuteAlarmMsg,
) -> Result<CwResponse, Infallible> {
    match msg {
        ExecuteAlarmMsg::TimeAlarm {} => Ok(response::empty_response()), // we just consume the time alarm
    }
}

#[entry_point]
pub fn query(_deps: Deps<'_>, _env: Env, EmptyMsg {}: EmptyMsg) -> Result<Binary, Infallible> {
    unimplemented!("No query is availabve on a Void contract!");
}
