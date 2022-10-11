use serde::{Deserialize, Serialize};

use sdk::{
    cosmwasm_std::{Addr, Order, StdResult, Storage, Timestamp},
    cw_storage_plus::{Bound, Index, IndexList, IndexedMap, Item, MultiIndex},
};

use crate::AlarmError;

type TimeSeconds = u64;
pub type Id = u64;

fn as_seconds(from: Timestamp) -> TimeSeconds {
    from.seconds()
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
struct Alarm {
    pub time: TimeSeconds,
    pub addr: Addr,
}

struct AlarmIndexes<'a> {
    alarms: MultiIndex<'a, TimeSeconds, Alarm, Id>,
}

impl<'a> IndexList<Alarm> for AlarmIndexes<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<Alarm>> + '_> {
        let v: Vec<&dyn Index<Alarm>> = vec![&self.alarms];
        Box::new(v.into_iter())
    }
}

pub struct Alarms<'a> {
    namespace_alarms: &'a str,
    namespace_index: &'a str,
    next_id: Item<'a, Id>,
}

impl<'a> Alarms<'a> {
    pub const fn new(
        namespace_alarms: &'a str,
        namespace_index: &'a str,
        namespace_next_id: &'a str,
    ) -> Self {
        Alarms {
            namespace_alarms,
            namespace_index,
            next_id: Item::new(namespace_next_id),
        }
    }

    fn alarms(&self) -> IndexedMap<TimeSeconds, Alarm, AlarmIndexes<'a>> {
        let indexes = AlarmIndexes {
            alarms: MultiIndex::new(|_, d| d.time, self.namespace_alarms, self.namespace_index),
        };
        IndexedMap::new(self.namespace_alarms, indexes)
    }

    pub fn add(&self, storage: &mut dyn Storage, addr: Addr, time: Timestamp) -> StdResult<Id> {
        let id = self.next_id.may_load(storage)?.unwrap_or_default();
        let alarm = Alarm {
            time: as_seconds(time),
            addr,
        };
        self.alarms().save(storage, id, &alarm)?;
        self.next_id.save(storage, &(id + 1))?;
        Ok(id)
    }

    pub fn remove(&self, storage: &mut dyn Storage, id: Id) -> StdResult<()> {
        self.alarms().remove(storage, id)
    }

    pub fn notify(
        &self,
        storage: &mut dyn Storage,
        dispatcher: &mut impl AlarmDispatcher,
        ctime: Timestamp,
    ) -> Result<(), AlarmError> {
        let max_id = self.next_id.may_load(storage)?.unwrap_or_default();

        let timestamps = self.alarms().idx.alarms.range(
            storage,
            None,
            Some(Bound::inclusive((as_seconds(ctime), max_id))),
            Order::Ascending,
        );
        for timestamp in timestamps {
            let (id, alarm) = timestamp?;
            dispatcher.send_to(id, alarm.addr, ctime)?;
        }

        Ok(())
    }
}

pub trait AlarmDispatcher {
    fn send_to(&mut self, id: Id, addr: Addr, ctime: Timestamp) -> Result<(), AlarmError>;
}

#[cfg(test)]
pub mod tests {
    use sdk::cosmwasm_std::testing;

    use super::*;

    #[derive(Default)]
    struct MockAlarmDispatcher(pub Vec<Id>);

    impl AlarmDispatcher for MockAlarmDispatcher {
        fn send_to(&mut self, id: Id, _addr: Addr, _ctime: Timestamp) -> Result<(), AlarmError> {
            self.0.push(id);
            Ok(())
        }
    }

    impl MockAlarmDispatcher {
        fn clean_alarms(&self, storage: &mut dyn Storage, alarms: &Alarms) -> StdResult<()> {
            for id in self.0.iter() {
                alarms.remove(storage, *id)?;
            }
            Ok(())
        }
    }

    #[test]
    fn test_add() {
        let alarms = Alarms::new("alarms", "alarms_idx", "alarms_next_id");
        let storage = &mut testing::mock_dependencies().storage;

        let t1 = Timestamp::from_seconds(1);
        let t2 = Timestamp::from_seconds(2);
        let addr1 = Addr::unchecked("addr1");
        let addr2 = Addr::unchecked("addr2");
        let addr3 = Addr::unchecked("addr3");

        assert_eq!(alarms.add(storage, addr1, t1), Ok(0));
        // same timestamp
        assert_eq!(alarms.add(storage, addr2, t1), Ok(1));
        // different timestamp
        assert_eq!(alarms.add(storage, addr3, t2), Ok(2));
    }

    #[test]
    fn test_remove() {
        let alarms = Alarms::new("alarms", "alarms_idx", "alarms_next_id");
        let storage = &mut testing::mock_dependencies().storage;
        let mut dispatcher = MockAlarmDispatcher::default();
        let t1 = Timestamp::from_seconds(1);
        let t2 = Timestamp::from_seconds(2);
        let addr1 = Addr::unchecked("addr1");
        let addr2 = Addr::unchecked("addr2");
        let addr3 = Addr::unchecked("addr3");
        let err_id = 4;

        // same time stamp
        let id1 = alarms.add(storage, addr1, t1).expect("can't set alarms");
        let id2 = alarms.add(storage, addr2, t1).expect("can't set alarms");
        // different timestamp
        let id3 = alarms.add(storage, addr3, t2).expect("can't set alarms");

        assert_eq!(alarms.remove(storage, id1), Ok(()));
        assert_eq!(alarms.remove(storage, id3), Ok(()));

        // unknown recipient: cw_storage_plus Map does't throw an Err, when removes unknown item.
        alarms
            .remove(storage, err_id)
            .expect("remove alarm with unknown id");

        assert_eq!(alarms.notify(storage, &mut dispatcher, t2), Ok(()));
        assert_eq!(dispatcher.0, [id2]);
    }

    #[test]
    fn test_notify() {
        let alarms = Alarms::new("alarms", "alarms_idx", "alarms_next_id");
        let storage = &mut testing::mock_dependencies().storage;
        let mut dispatcher = MockAlarmDispatcher::default();
        let t1 = Timestamp::from_seconds(1);
        let t2 = Timestamp::from_seconds(2);
        let t3 = Timestamp::from_seconds(3);
        let t4 = Timestamp::from_seconds(4);
        let addr1 = Addr::unchecked("addr1");
        let addr2 = Addr::unchecked("addr2");
        let addr3 = Addr::unchecked("addr3");
        let addr4 = Addr::unchecked("addr4");

        // same timestamp
        let id1 = alarms.add(storage, addr1, t1).expect("can't set alarms");
        let id2 = alarms.add(storage, addr2, t1).expect("can't set alarms");
        // different timestamp
        let id3 = alarms.add(storage, addr3, t2).expect("can't set alarms");
        // rest
        alarms.add(storage, addr4, t4).expect("can't set alarms");

        assert_eq!(alarms.notify(storage, &mut dispatcher, t1), Ok(()));
        assert_eq!(dispatcher.0, [id1, id2]);
        dispatcher
            .clean_alarms(storage, &alarms)
            .expect("can't clean up alarms db");

        let mut dispatcher = MockAlarmDispatcher::default();
        assert_eq!(alarms.notify(storage, &mut dispatcher, t3), Ok(()));
        assert_eq!(dispatcher.0, [id3]);
    }
}
