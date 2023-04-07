use serde::{Deserialize, Serialize};

use sdk::{
    cosmwasm_std::{Addr, Storage, Timestamp},
    cw_storage_plus::{Index, IndexList, IndexedMap, Item, MultiIndex},
};

use crate::AlarmError;

use super::Alarms;

type TimeSeconds = u64;
type Id = u64;

#[cfg(test)]
fn as_seconds(from: Timestamp) -> TimeSeconds {
    from.seconds()
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

pub struct AlarmsOld<'a> {
    namespace_alarms: &'a str,
    namespace_index: &'a str,
    next_id: Item<'a, Id>,
}

impl<'a> AlarmsOld<'a> {
    pub const fn new(
        namespace_alarms: &'a str,
        namespace_index: &'a str,
        namespace_next_id: &'a str,
    ) -> Self {
        Self {
            namespace_alarms,
            namespace_index,
            next_id: Item::new(namespace_next_id),
        }
    }

    pub fn migrate(
        &self,
        storage: &mut dyn Storage,
        alarms_new: &Alarms,
    ) -> Result<(), AlarmError> {
        // const BATCH_SIZE: u32 = 100;
        const PROFIT_ALARM_KEY: u64 = 0;
        const REWARDS_ALARM_KEY: u64 = 1;

        let old_alarms = self.alarms();
        migrate_alarm(storage, &old_alarms, alarms_new, PROFIT_ALARM_KEY)?;
        migrate_alarm(storage, &old_alarms, alarms_new, REWARDS_ALARM_KEY)?;

        // loop {
        //     let keys: Vec<_> = old_alarms
        //         .keys(storage, None, None, Order::Ascending)
        //         .take(BATCH_SIZE.try_into().unwrap())
        //         .map(Result::unwrap)
        //         .collect();
        //     if keys.is_empty() {
        //         break;
        //     }

        //     keys.iter()
        //         .try_for_each(|key| old_alarms.remove(storage, *key))?;
        // }

        self.next_id.remove(storage);

        Ok(())
    }

    #[cfg(test)]
    fn add(
        &self,
        storage: &mut dyn Storage,
        addr: Addr,
        time: Timestamp,
    ) -> Result<(), AlarmError> {
        let id = self.next_id.may_load(storage)?.unwrap_or_default();

        let alarm = AlarmOld {
            time: as_seconds(time),
            addr,
        };

        self.alarms().save(storage, id, &alarm)?;

        self.next_id.save(storage, &id.wrapping_add(1))?;

        Ok(())
    }

    fn alarms(&self) -> IndexedMap<'a, TimeSeconds, AlarmOld, AlarmIndexes<'a>> {
        let indexes = AlarmIndexes {
            alarms: MultiIndex::new(|_, d| d.time, self.namespace_alarms, self.namespace_index),
        };

        IndexedMap::new(self.namespace_alarms, indexes)
    }
}

fn migrate_alarm(
    storage: &mut dyn Storage,
    old_alarms: &IndexedMap<u64, AlarmOld, AlarmIndexes>,
    alarms_new: &Alarms,
    alarm_key: u64,
) -> Result<(), AlarmError> {
    let alarm = old_alarms.may_load(storage, alarm_key);
    alarm.map_err(Into::into).and_then(|may_alarm| {
        if let Some(alarm) = may_alarm {
            alarms_new.add(storage, alarm.addr, Timestamp::from_seconds(alarm.time))
        } else {
            Ok(())
        }
    })
}

#[cfg(test)]
pub mod tests {
    use sdk::cosmwasm_std::{testing, Order};

    use super::*;

    #[test]
    fn test_migration() {
        let alarms = AlarmsOld::new("alarms", "alarms_idx", "alarms_next_id");
        let storage = &mut testing::mock_dependencies().storage;
        let t1 = 1;
        let t2 = 2;
        let t3 = 3;
        let t4 = 4;
        let addr1 = Addr::unchecked("addr1");
        let addr2 = Addr::unchecked("addr2");
        let addr3 = Addr::unchecked("addr3");
        let addr4 = Addr::unchecked("addr4");

        alarms
            .add(storage, addr1.clone(), Timestamp::from_seconds(t1))
            .unwrap();
        alarms
            .add(storage, addr2.clone(), Timestamp::from_seconds(t1))
            .unwrap();
        alarms
            .add(storage, addr3.clone(), Timestamp::from_seconds(t2))
            .unwrap();
        alarms
            .add(storage, addr3.clone(), Timestamp::from_seconds(t3))
            .unwrap();
        alarms
            .add(storage, addr4.clone(), Timestamp::from_seconds(t4))
            .unwrap();

        // multiple alarms per address(5) + index(5) + next_id(1)
        assert_eq!(storage.range(None, None, Order::Ascending).count(), 11);
        let alarms_new = Alarms::new("new_alarms", "new_alarms_idx");
        alarms.migrate(storage, &alarms_new).unwrap();

        // single alarm per address(4) + index(4)
        assert_eq!(storage.range(None, None, Order::Ascending).count(), 8);

        let new_alarms = Alarms::new("new_alarms", "new_alarms_idx");
        let result: Vec<_> = new_alarms
            .alarms_selection(storage, Timestamp::from_seconds(10))
            .map(Result::unwrap)
            .collect();

        assert_eq!(
            result,
            vec![(addr1, t1), (addr2, t1), (addr3, t2), (addr4, t4)]
        );
    }
}
