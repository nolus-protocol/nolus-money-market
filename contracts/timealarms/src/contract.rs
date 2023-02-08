#[cfg(feature = "contract-with-bindings")]
use sdk::cosmwasm_std::entry_point;
use sdk::{
    cosmwasm_ext::Response,
    cosmwasm_std::{Binary, Deps, DepsMut, Env, MessageInfo, Reply, SubMsgResult},
};
use versioning::{version, VersionSegment};

use crate::{
    alarms::TimeAlarms,
    error::ContractError,
    msg::{ExecuteMsg, InstantiateMsg, QueryMsg},
};

// version info for migration info
// const CONTRACT_STORAGE_VERSION_FROM: VersionSegment = 0;
const CONTRACT_STORAGE_VERSION: VersionSegment = 0;

#[cfg_attr(feature = "contract-with-bindings", entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    versioning::initialize(deps.storage, version!(CONTRACT_STORAGE_VERSION))?;

    Ok(Response::default())
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct MigrateMsg {}

#[cfg_attr(feature = "contract-with-bindings", entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    versioning::upgrade_old_contract::<0, fn(_) -> _, ContractError>(
        deps.storage,
        version!(CONTRACT_STORAGE_VERSION),
        None,
    )?;

    Ok(Response::default())
}

#[cfg_attr(feature = "contract-with-bindings", entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::AddAlarm { time } => TimeAlarms::try_add(deps, env, info.sender, time),
        ExecuteMsg::DispatchAlarms { max_count } => {
            TimeAlarms::try_notify(deps.storage, env.block.time, max_count)
        }
    }
}

#[cfg_attr(feature = "contract-with-bindings", entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    match msg {
        QueryMsg::AlarmsStatus {} => Ok(sdk::cosmwasm_std::to_binary(&TimeAlarms::try_any_alarm(
            deps.storage,
            env.block.time,
        )?)?),
    }
}

#[cfg_attr(feature = "contract-with-bindings", entry_point)]
pub fn reply(deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, ContractError> {
    let res = match msg.result {
        SubMsgResult::Ok(_) => {
            TimeAlarms::remove(deps.storage, msg.id)?;
            Response::new().add_attribute("alarm", "success")
        }
        SubMsgResult::Err(err) => Response::new()
            .add_attribute("alarm", "error")
            .add_attribute("error", err),
    };
    Ok(res)
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
