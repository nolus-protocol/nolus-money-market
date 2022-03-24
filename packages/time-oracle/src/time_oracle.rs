use cosmwasm_std::{StdResult, Storage, Timestamp};
use cw_storage_plus::Item;

pub struct TimeOracle<'a>(Item<'a, Timestamp>);

impl<'a> TimeOracle<'a> {
    pub const fn new(namespace: &'a str) -> Self {
        TimeOracle(Item::new(namespace))
    }

    pub fn update_global_time(&self, storage: &mut dyn Storage, time: Timestamp) -> StdResult<()> {
        self.0.save(storage, &time)?;
        Ok(())
    }

    pub fn query_global_time(&self, storage: &dyn Storage) -> StdResult<Timestamp> {
        let time = self.0.load(storage)?;
        Ok(time)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing;

    #[test]
    fn test_update_and_query_global_time() {
        let time_oracle = TimeOracle::new("time_oracle");
        let mut deps = testing::mock_dependencies();
        let timestamp = Timestamp::from_seconds(1);

        time_oracle
            .update_global_time(&mut deps.storage, timestamp)
            .expect("can't update global time");

        let time_response = time_oracle
            .query_global_time(&deps.storage)
            .expect("can't query global time");

        assert_eq!(timestamp, time_response);
    }
}
