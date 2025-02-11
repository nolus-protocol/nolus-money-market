use std::convert::Infallible;

use platform::response;
use sdk::{
    cosmwasm_ext::Response as CwResponse,
    cosmwasm_std::{self, entry_point, Binary, Deps, DepsMut, Env, MessageInfo},
};
use timealarms::msg::ExecuteAlarmMsg;
use versioning::{
    self, package_name, package_version, ProtocolPackageRelease, ReleaseId, VersionSegment,
};

use crate::{
    error::Error,
    msg::{EmptyMsg, QueryMsg},
};

mod error;
mod msg;

const CONTRACT_STORAGE_VERSION: VersionSegment = 0;
const CURRENT_RELEASE: ProtocolPackageRelease = ProtocolPackageRelease::current(
    package_name!(),
    package_version!(),
    CONTRACT_STORAGE_VERSION,
);

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
    response::response(ReleaseId::VOID)
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
pub fn query(_deps: Deps<'_>, _env: Env, msg: QueryMsg) -> Result<Binary, Error> {
    match msg {
        QueryMsg::ProtocolPackageRelease {} => {
            cosmwasm_std::to_json_binary(&CURRENT_RELEASE).map_err(Into::into)
        }
    }
}
