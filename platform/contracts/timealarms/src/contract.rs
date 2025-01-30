use platform::{
    batch::{Emit, Emitter},
    error as platform_error, response,
};
use sdk::{
    cosmwasm_ext::Response as CwResponse,
    cosmwasm_std::{
        entry_point, to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Reply, Storage,
        SubMsgResult,
    },
};
use versioning::{
    package_name, package_version, PlatformMigrationMessage, PlatformPackageRelease,
    UpdatablePackage as _, VersionSegment,
};

use crate::{
    alarms::TimeAlarms,
    msg::{DispatchAlarmsResponse, ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg, SudoMsg},
    result::ContractResult,
};

const CONTRACT_STORAGE_VERSION: VersionSegment = 1;
const CURRENT_RELEASE: PlatformPackageRelease = PlatformPackageRelease::current(
    package_name!(),
    package_version!(),
    CONTRACT_STORAGE_VERSION,
);

#[entry_point]
pub fn instantiate(
    _deps: DepsMut<'_>,
    _env: Env,
    _info: MessageInfo,
    _msg: InstantiateMsg,
) -> ContractResult<CwResponse> {
    Ok(response::empty_response())
}

#[entry_point]
pub fn migrate(
    deps: DepsMut<'_>,
    _env: Env,
    PlatformMigrationMessage {
        to_release,
        message: MigrateMsg {},
    }: PlatformMigrationMessage<MigrateMsg>,
) -> ContractResult<CwResponse> {
    PlatformPackageRelease::pull_prev(package_name!(), deps.storage)
        .and_then(|previous| previous.update_software(&CURRENT_RELEASE, &to_release))
        .map_err(Into::into)
        .and_then(response::response)
        .inspect_err(platform_error::log(deps.api))
}

#[entry_point]
pub fn execute(
    deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> ContractResult<CwResponse> {
    let mut time_alarms: TimeAlarms<'_, &mut dyn Storage> = TimeAlarms::new(deps.storage);

    match msg {
        ExecuteMsg::AddAlarm { time } => time_alarms
            .try_add(deps.querier, &env, info.sender, time)
            .map(response::response_only_messages),
        ExecuteMsg::DispatchAlarms { max_count } => time_alarms
            .try_notify(env.block.time, max_count)
            .and_then(|(total, resp)| {
                response::response_with_messages(DispatchAlarmsResponse(total), resp)
            }),
    }
    .inspect_err(platform_error::log(deps.api))
}

#[entry_point]
pub fn sudo(_deps: DepsMut<'_>, _env: Env, msg: SudoMsg) -> ContractResult<CwResponse> {
    match msg {}
}

#[entry_point]
pub fn query(deps: Deps<'_>, env: Env, msg: QueryMsg) -> ContractResult<Binary> {
    match msg {
        QueryMsg::ContractVersion {} => Ok(to_json_binary(&CURRENT_RELEASE.version())?),
        QueryMsg::AlarmsStatus {} => Ok(to_json_binary(
            &TimeAlarms::new(deps.storage).try_any_alarm(env.block.time)?,
        )?),
        QueryMsg::PlatformPackageRelease {} => {
            cosmwasm_std::to_json_binary(&CURRENT_RELEASE).map_err(Into::into)
        }
    }
    .inspect_err(platform_error::log(deps.api))
}

#[entry_point]
pub fn reply(deps: DepsMut<'_>, env: Env, msg: Reply) -> ContractResult<CwResponse> {
    const EVENT_TYPE: &str = "time-alarm";
    const KEY_DELIVERED: &str = "delivered";
    const KEY_DETAILS: &str = "details";

    let emitter: Emitter = Emitter::of_type(EVENT_TYPE);

    let mut time_alarms: TimeAlarms<'_, &mut dyn Storage> = TimeAlarms::new(deps.storage);

    Ok(response::response_only_messages(match msg.result {
        SubMsgResult::Ok(_) => {
            time_alarms.last_delivered()?;

            emitter.emit(KEY_DELIVERED, "success")
        }
        SubMsgResult::Err(err) => {
            time_alarms.last_failed(env.block.time)?;

            emitter.emit(KEY_DELIVERED, "error").emit(KEY_DETAILS, err)
        }
    }))
}

#[cfg(test)]
mod tests {
    use sdk::cosmwasm_std::{
        self,
        testing::{mock_dependencies, mock_env},
        Addr, MessageInfo,
    };

    use crate::msg::InstantiateMsg;

    use super::instantiate;

    #[test]
    fn proper_initialization() {
        let msg = InstantiateMsg {};
        let mut deps = mock_dependencies();
        let info = MessageInfo {
            sender: Addr::unchecked("CREATOR"),
            funds: vec![cosmwasm_std::coin(1000, "token")],
        };
        instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
    }
}
