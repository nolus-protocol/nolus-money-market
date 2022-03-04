use cosmwasm_std::{Deps, Env, Response, StdResult, Storage, Timestamp};
use cw_storage_plus::Item;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

const GLOBAL_TIME: Item<Timestamp> = Item::new("Global time");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct GlobalTimeResponse {
    pub time: Timestamp,
}

pub fn update_global_time(storage: &mut dyn Storage, env: &Env) -> StdResult<Response> {
    let time = &env.block.time;
    GLOBAL_TIME.save(storage, time)?;
    Ok(Response::new().add_attribute("method", "update_time"))
}

pub fn query_global_time(deps: Deps) -> StdResult<GlobalTimeResponse> {
    let time = GLOBAL_TIME.load(deps.storage)?;
    Ok(GlobalTimeResponse { time })
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies_with_balance, mock_env, mock_info};
    use cosmwasm_std::{coins, from_binary, to_binary, Binary, DepsMut, MessageInfo};

    // simplified version of smart contract
    enum ExecuteMsg {
        FeedPrice,
    }
    enum QueryMsg {
        GlobalTime,
    }

    fn execute(
        deps: DepsMut,
        env: Env,
        _info: MessageInfo,
        _msg: ExecuteMsg,
    ) -> StdResult<Response> {
        update_global_time(deps.storage, &env)
    }

    fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
        match msg {
            QueryMsg::GlobalTime => to_binary(&query_global_time(deps)?),
        }
    }

    #[test]
    fn test_update_and_query_global_time() {
        let mut deps = mock_dependencies_with_balance(&coins(10, "unolus"));
        let info = mock_info("feeder", &coins(10, "unolus"));
        let msg = ExecuteMsg::FeedPrice;
        let env = mock_env();

        execute(deps.as_mut(), env.clone(), info, msg).expect("can't update global time");

        let msg = QueryMsg::GlobalTime;

        let res = query(deps.as_ref(), mock_env(), msg).expect("can't query global time");
        let time_response: GlobalTimeResponse =
            from_binary(&res).expect("can't deserialize GlobalTimeResponse");

        assert_eq!(env.block.time, time_response.time);
    }
}
