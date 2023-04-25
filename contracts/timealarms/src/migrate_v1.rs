use cosmwasm_std::{Addr, Order, Storage};
use sdk::cw_storage_plus::{Bound, Index, IndexList, IndexedMap, MultiIndex};
use serde::{Deserialize, Serialize};

use crate::ContractError;

// const PROFIT_ADDR: &str = "nolus1mf6ptkssddfmxvhdx0ech0k03ktp6kf9yk59renau2gvht3nq2gqkxgywu";
// const REWARDS_DISPATCHER_ADDR: &str =
//     "nolus1tqwwyth34550lg2437m05mjnjp8w7h5ka7m70jtzpxn4uh2ktsmqtctwnn";
// const ALARM_ON: Timestamp = Timestamp::from_seconds(1701263948);

pub fn cleanup_v0(storage: &mut dyn Storage, delete_batch_size: u32) -> Result<(), ContractError> {
    let old_alarms = old_alarms();

    let keys: Vec<_> = old_alarms
        .keys(
            storage,
            Some(Bound::inclusive(0u64)),
            Some(Bound::inclusive(TimeSeconds::MAX)),
            Order::Ascending,
        )
        .take(delete_batch_size.try_into().unwrap())
        .map(Result::unwrap)
        .collect();

    keys.iter()
        .try_for_each(|key| old_alarms.remove(storage, *key))
        .map_err(Into::into)
    // alarms
    //     .add(storage, Addr::unchecked(PROFIT_ADDR), ALARM_ON)
    //     .and_then(|()| alarms.add(storage, Addr::unchecked(REWARDS_DISPATCHER_ADDR), ALARM_ON))
    //     .map_err(Into::into)
}

type TimeSeconds = u64;
type Id = u64;
const NS_ALARMS: &str = "alarms";
const NS_ALARM_IDX: &str = "alarms_idx";

fn old_alarms<'a>() -> IndexedMap<'a, TimeSeconds, AlarmOld, AlarmIndexes<'a>> {
    let indexes = AlarmIndexes {
        alarms: MultiIndex::new(|_, d| d.time, NS_ALARMS, NS_ALARM_IDX),
    };

    IndexedMap::new(NS_ALARMS, indexes)
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
struct AlarmOld {
    pub time: TimeSeconds,
    pub addr: Addr,
}

struct AlarmIndexes<'a> {
    alarms: MultiIndex<'a, TimeSeconds, AlarmOld, Id>,
}

impl<'a> IndexList<AlarmOld> for AlarmIndexes<'a> {
    fn get_indexes(&self) -> Box<dyn Iterator<Item = &'_ dyn Index<AlarmOld>> + '_> {
        let v: Vec<&dyn Index<AlarmOld>> = vec![&self.alarms];

        Box::new(v.into_iter())
    }
}
