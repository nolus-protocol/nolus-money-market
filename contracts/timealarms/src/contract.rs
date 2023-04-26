use platform::{
    batch::{Emit, Emitter},
    reply,
    response::{self},
};
#[cfg(feature = "contract-with-bindings")]
use sdk::cosmwasm_std::entry_point;
use sdk::{
    cosmwasm_ext::Response as CwResponse,
    cosmwasm_std::{to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Reply},
};
use versioning::{package_version, version, VersionSegment};

use crate::{
    alarms::TimeAlarms,
    msg::{DispatchAlarmsResponse, ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg, SudoMsg},
    result::ContractResult,
    ContractError,
};

// version info for migration info
// const CONTRACT_STORAGE_VERSION_FROM: VersionSegment = 0;
const CONTRACT_STORAGE_VERSION: VersionSegment = 1;

#[cfg_attr(feature = "contract-with-bindings", entry_point)]
pub fn instantiate(
    deps: DepsMut<'_>,
    _env: Env,
    _info: MessageInfo,
    _msg: InstantiateMsg,
) -> ContractResult<CwResponse> {
    versioning::initialize(deps.storage, version!(CONTRACT_STORAGE_VERSION))?;

    Ok(response::empty_response())
}

#[cfg_attr(feature = "contract-with-bindings", entry_point)]
pub fn migrate(deps: DepsMut<'_>, _env: Env, _msg: MigrateMsg) -> ContractResult<CwResponse> {
    versioning::update_software(deps.storage, version!(CONTRACT_STORAGE_VERSION))
        .map_err(Into::into)
        .and_then(|label| response::response(&label))
}

#[cfg_attr(feature = "contract-with-bindings", entry_point)]
pub fn execute(
    deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> ContractResult<CwResponse> {
    match msg {
        ExecuteMsg::AddAlarm { time } => TimeAlarms::new()
            .try_add(deps, env, info.sender, time)
            .map(response::response_only_messages),
        ExecuteMsg::DispatchAlarms { max_count } => TimeAlarms::new()
            .try_notify(deps.storage, env.block.time, max_count)
            .and_then(|(total, resp)| {
                response::response_with_messages::<_, _, ContractError>(
                    &DispatchAlarmsResponse(total),
                    resp,
                )
            }),
    }
}

#[cfg_attr(feature = "contract-with-bindings", entry_point)]
pub fn sudo(deps: DepsMut<'_>, _env: Env, msg: SudoMsg) -> ContractResult<CwResponse> {
    match msg {
        SudoMsg::RemoveTimeAlarm { receiver } => TimeAlarms::new()
            .remove(deps.storage, receiver)
            .map(|()| Default::default()),
    }
}

#[cfg_attr(feature = "contract-with-bindings", entry_point)]
pub fn query(deps: Deps<'_>, env: Env, msg: QueryMsg) -> ContractResult<Binary> {
    match msg {
        QueryMsg::ContractVersion {} => Ok(to_binary(&package_version!())?),
        QueryMsg::AlarmsStatus {} => Ok(to_binary(
            &TimeAlarms::new().try_any_alarm(deps.storage, env.block.time)?,
        )?),
    }
}

#[cfg_attr(feature = "contract-with-bindings", entry_point)]
pub fn reply(deps: DepsMut<'_>, _env: Env, msg: Reply) -> ContractResult<CwResponse> {
    const EVENT_TYPE: &str = "time-alarm";
    const KEY_DELIVERED: &str = "delivered";
    const KEY_DETAILS: &str = "details";

    match reply::from_execute(msg) {
        Ok(Some(addr)) => TimeAlarms::new()
            .remove(deps.storage, addr)
            .map(|()| Emitter::of_type(EVENT_TYPE).emit(KEY_DELIVERED, "success")),
        Err(err) => Ok(Emitter::of_type(EVENT_TYPE)
            .emit(KEY_DELIVERED, "error")
            .emit(KEY_DETAILS, err.to_string())),

        Ok(None) => Ok(Emitter::of_type(EVENT_TYPE)
            .emit(KEY_DELIVERED, "error")
            .emit(KEY_DETAILS, "no reply")),
    }
    .map(response::response_only_messages)
}

#[cfg(test)]
mod tests {
    use sdk::cosmwasm_std::{
        coins,
        testing::{mock_dependencies, mock_env, mock_info},
    };

    use crate::msg::InstantiateMsg;

    use super::instantiate;

    #[test]
    fn proper_initialization() {
        let msg = InstantiateMsg {};
        let mut deps = mock_dependencies();
        let info = mock_info("CREATOR", &coins(1000, "token"));
        instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
    }
}
