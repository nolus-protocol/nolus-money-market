use crate::alarms::{self, MsgSender};
use cosmwasm_std::{Env, Response, StdResult, Storage, Timestamp};
use cw_storage_plus::Item;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

const GLOBAL_TIME: Item<Timestamp> = Item::new("Global time");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct GlobalTimeResponse {
    pub time: Timestamp,
}

pub fn update_global_time(
    storage: &mut dyn Storage,
    env: &Env,
    sender: &impl MsgSender,
) -> StdResult<Response> {
    let time = env.block.time;
    GLOBAL_TIME.save(storage, &time)?;
    alarms::notify(storage, sender, time)?;
    Ok(Response::new().add_attribute("method", "update_time"))
}

pub fn query_global_time(storage: &dyn Storage) -> StdResult<GlobalTimeResponse> {
    let time = GLOBAL_TIME.load(storage)?;
    Ok(GlobalTimeResponse { time })
}

#[cfg(test)]
mod tests {
    use super::*;
    use alarms::tests::MockSender;
    use cosmwasm_std::testing;

    #[test]
    fn test_update_and_query_global_time() {
        let mut deps = testing::mock_dependencies_with_balance(&cosmwasm_std::coins(10, "unolus"));
        let env = testing::mock_env();
        let sender = MockSender::new();

        update_global_time(&mut deps.storage, &env, &sender).expect("can't update global time");

        let time_response = query_global_time(&deps.storage).expect("can't query global time");

        assert_eq!(env.block.time, time_response.time);
    }
}
