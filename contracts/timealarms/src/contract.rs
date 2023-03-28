use platform::{reply, response};
#[cfg(feature = "contract-with-bindings")]
use sdk::cosmwasm_std::entry_point;
use sdk::{
    cosmwasm_ext::Response,
    cosmwasm_std::{to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Reply},
};
use versioning::{package_version, version, VersionSegment};

use crate::{
    alarms::TimeAlarms,
    error::ContractError,
    msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg, SudoMsg},
};

// version info for migration info
const CONTRACT_STORAGE_VERSION_FROM: VersionSegment = 0;
const CONTRACT_STORAGE_VERSION: VersionSegment = 1;

#[cfg_attr(feature = "contract-with-bindings", entry_point)]
pub fn instantiate(
    deps: DepsMut<'_>,
    _env: Env,
    _info: MessageInfo,
    _msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    versioning::initialize(deps.storage, version!(CONTRACT_STORAGE_VERSION))?;

    Ok(Response::default())
}

#[cfg_attr(feature = "contract-with-bindings", entry_point)]
pub fn migrate(deps: DepsMut<'_>, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    versioning::update_software_and_storage::<CONTRACT_STORAGE_VERSION_FROM, _, _>(
        deps.storage,
        version!(CONTRACT_STORAGE_VERSION),
        |storage: &mut _| TimeAlarms::migrate_v1(storage),
    )?;

    response::response(versioning::release()).map_err(Into::into)
}

#[cfg_attr(feature = "contract-with-bindings", entry_point)]
pub fn execute(
    deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::AddAlarm { time } => TimeAlarms::new().try_add(deps, env, info.sender, time),
        ExecuteMsg::DispatchAlarms { max_count } => {
            TimeAlarms::new().try_notify(deps.storage, env.block.time, max_count)
        }
    }
}

#[cfg_attr(feature = "contract-with-bindings", entry_point)]
pub fn sudo(deps: DepsMut<'_>, _env: Env, msg: SudoMsg) -> Result<Response, ContractError> {
    match msg {
        SudoMsg::RemoveTimeAlarm { receiver } => {
            TimeAlarms::new().remove(deps.storage, receiver)?;
            Ok(Response::default())
        }
    }
}

#[cfg_attr(feature = "contract-with-bindings", entry_point)]
pub fn query(deps: Deps<'_>, env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    match msg {
        QueryMsg::ContractVersion {} => Ok(to_binary(&package_version!())?),
        QueryMsg::AlarmsStatus {} => Ok(to_binary(
            &TimeAlarms::new().try_any_alarm(deps.storage, env.block.time)?,
        )?),
    }
}

#[cfg_attr(feature = "contract-with-bindings", entry_point)]
pub fn reply(deps: DepsMut<'_>, _env: Env, msg: Reply) -> Result<Response, ContractError> {
    let resp = match reply::from_execute(msg) {
        Ok(Some(addr)) => {
            TimeAlarms::new().remove(deps.storage, addr)?;
            Response::new().add_attribute("alarm", "success")
        }
        Err(err) => Response::new()
            .add_attribute("alarm", "error")
            .add_attribute("error", err.to_string()),

        Ok(None) => Response::new()
            .add_attribute("alarm", "error")
            .add_attribute("error", "No data"),
    };

    Ok(resp)
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
