use cosmwasm_std::{StdResult, Storage, Timestamp};
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
    time: Timestamp,
) -> StdResult<()> {
    GLOBAL_TIME.save(storage, &time)?;
    Ok(())
}

pub fn query_global_time(storage: &dyn Storage) -> StdResult<GlobalTimeResponse> {
    let time = GLOBAL_TIME.load(storage)?;
    Ok(GlobalTimeResponse { time })
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing;

    #[test]
    fn test_update_and_query_global_time() {
        let mut deps = testing::mock_dependencies();
        let timestamp = Timestamp::from_seconds(1);

        update_global_time(&mut deps.storage, timestamp).expect("can't update global time");

        let time_response = query_global_time(&deps.storage).expect("can't query global time");

        assert_eq!(timestamp, time_response.time);
    }
}
