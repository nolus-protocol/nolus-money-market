use serde::{Deserialize, Serialize};

use sdk::{
    cosmwasm_std::{Addr, Order, StdError, Storage, Timestamp},
    cw_storage_plus::{Index, IndexList, IndexedMap, Item, MultiIndex},
};

use super::Alarms;
use crate::AlarmError;

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

    pub fn migrate(&self, storage: &mut dyn Storage) -> Result<(), AlarmError> {
        let old_alarms = self.alarms();

        let (alarms, ids) = old_alarms
            .idx
            .alarms
            .range(storage, None, None, Order::Descending)
            .try_fold(
                (vec![], vec![]),
                |mut v: (Vec<AlarmOld>, Vec<Id>), alarm| -> Result<_, StdError> {
                    let alarm = alarm?;
                    v.1.push(alarm.0);
                    if let Some(last) = v.0.last() {
                        if last.addr != alarm.1.addr {
                            v.0.push(alarm.1)
                        }
                    } else {
                        v.0.push(alarm.1)
                    }
                    Ok(v)
                },
            )?;

        // purge all the data
        for id in ids {
            old_alarms.remove(storage, id)?;
        }
        self.next_id.remove(storage);

        let alarms_new = Alarms::new(self.namespace_alarms, self.namespace_index);

        // restore to new alarms
        for alarm in alarms {
            alarms_new.add(storage, alarm.addr, Timestamp::from_seconds(alarm.time))?;
        }

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

#[cfg(test)]
pub mod tests {
    use sdk::cosmwasm_std::testing;

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
        assert_eq!(11, storage.range(None, None, Order::Ascending).count());

        alarms.migrate(storage).unwrap();

        // single alarm per address(4) + index(4)
        assert_eq!(8, storage.range(None, None, Order::Ascending).count());

        let new_alarms = Alarms::new("alarms", "alarms_idx");
        let result: Vec<_> = new_alarms
            .alarms_selection(storage, Timestamp::from_seconds(10))
            .map(Result::unwrap)
            .collect();

        assert_eq!(
            result,
            vec![(addr1, t1), (addr2, t1), (addr3, t3), (addr4, t4),]
        );
    }
}
