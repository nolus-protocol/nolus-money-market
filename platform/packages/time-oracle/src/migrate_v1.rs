use serde::{Deserialize, Serialize};

use sdk::{
    cosmwasm_std::{Addr, Order, StdError, Storage, Timestamp},
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

struct AlarmIndexes {
    alarms: MultiIndex<'static, TimeSeconds, AlarmOld, Id>,
}

impl IndexList<AlarmOld> for AlarmIndexes {
    fn get_indexes(&self) -> Box<dyn Iterator<Item = &'_ dyn Index<AlarmOld>> + '_> {
        let v: Vec<&dyn Index<AlarmOld>> = vec![&self.alarms];

        Box::new(v.into_iter())
    }
}

pub struct AlarmsOld {
    namespace_alarms: &'static str,
    namespace_index: &'static str,
    namespace_in_delivery: &'static str,
    next_id: Item<Id>,
}

impl AlarmsOld {
    pub const fn new(
        namespace_alarms: &'static str,
        namespace_index: &'static str,
        namespace_in_delivery: &'static str,
        namespace_next_id: &'static str,
    ) -> Self {
        Self {
            namespace_alarms,
            namespace_index,
            namespace_in_delivery,
            next_id: Item::new(namespace_next_id),
        }
    }

    pub fn migrate(&self, storage: &mut dyn Storage) -> Result<(), AlarmError> {
        let old_alarms = self.alarms();

        let (alarms, ids) = old_alarms
            .idx
            .alarms
            .range(storage, None, None, Order::Ascending)
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

        let mut alarms_new = Alarms::new(
            storage,
            self.namespace_alarms,
            self.namespace_index,
            self.namespace_in_delivery,
        );

        // restore to new alarms
        for alarm in alarms {
            alarms_new.add(alarm.addr, Timestamp::from_seconds(alarm.time))?;
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

    fn alarms(&self) -> IndexedMap<TimeSeconds, AlarmOld, AlarmIndexes> {
        let indexes = AlarmIndexes {
            alarms: MultiIndex::new(|_, d| d.time, self.namespace_alarms, self.namespace_index),
        };

        IndexedMap::new(self.namespace_alarms, indexes)
    }
}

#[cfg(test)]
pub mod tests {
    use sdk::cosmwasm_std::testing::MockStorage;

    use super::{super::Alarms, *};

    #[test]
    fn test_migration() {
        let mut storage = MockStorage::new();
        let alarms = AlarmsOld::new("alarms", "alarms_idx", "in_delivery", "alarms_next_id");

        let t1 = 1;
        let t2 = 2;
        let t3 = 3;
        let t4 = 4;
        let addr1 = Addr::unchecked("addr1");
        let addr2 = Addr::unchecked("addr2");
        let addr3 = Addr::unchecked("addr3");
        let addr4 = Addr::unchecked("addr4");

        alarms
            .add(&mut storage, addr1.clone(), Timestamp::from_seconds(t1))
            .unwrap();
        alarms
            .add(&mut storage, addr2.clone(), Timestamp::from_seconds(t1))
            .unwrap();
        alarms
            .add(&mut storage, addr3.clone(), Timestamp::from_seconds(t2))
            .unwrap();
        alarms
            .add(&mut storage, addr3.clone(), Timestamp::from_seconds(t3))
            .unwrap();
        alarms
            .add(&mut storage, addr4.clone(), Timestamp::from_seconds(t4))
            .unwrap();

        // multiple alarms per address(5) + index(5) + next_id(1)
        assert_eq!(11, storage.range(None, None, Order::Ascending).count());

        alarms.migrate(&mut storage).unwrap();

        // single alarm per address(4) + index(4)
        assert_eq!(8, storage.range(None, None, Order::Ascending).count());

        let new_alarms = Alarms::new(
            &mut storage as &mut dyn Storage,
            "alarms",
            "alarms_idx",
            "in_delivery",
        );
        let result: Vec<_> = new_alarms
            .alarms_selection(Timestamp::from_seconds(10))
            .map(Result::unwrap)
            .collect();

        assert_eq!(result, vec![addr1, addr2, addr3, addr4]);
    }
}
