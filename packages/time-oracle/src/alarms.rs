use serde::{Deserialize, Serialize};

use sdk::{
    cosmwasm_std::{Addr, Order, Storage, Timestamp},
    cw_storage_plus::{Bound, Deque, Index, IndexList, IndexedMap, MultiIndex},
};

use crate::AlarmError;

type TimeSeconds = u64;

fn as_seconds(from: Timestamp) -> TimeSeconds {
    from.seconds()
}

struct AlarmIndexes<'a> {
    alarms: MultiIndex<'a, TimeSeconds, TimeSeconds, Addr>,
}

impl<'a> IndexList<TimeSeconds> for AlarmIndexes<'a> {
    fn get_indexes(&self) -> Box<dyn Iterator<Item = &'_ dyn Index<TimeSeconds>> + '_> {
        let v: Vec<&dyn Index<TimeSeconds>> = vec![&self.alarms];

        Box::new(v.into_iter())
    }
}

pub struct Alarms<'a> {
    alarms: IndexedMap<'a, Addr, TimeSeconds, AlarmIndexes<'a>>,
}

impl<'a> Alarms<'a> {
    pub fn new(namespace_alarms: &'a str, namespace_index: &'a str) -> Self {
        let indexes = AlarmIndexes {
            alarms: MultiIndex::new(|_, d| *d, namespace_alarms, namespace_index),
        };

        let alarms = IndexedMap::new(namespace_alarms, indexes);

        Self { alarms }
    }

    pub fn add(
        &self,
        storage: &mut dyn Storage,
        subscriber: Addr,
        time: Timestamp,
    ) -> Result<(), AlarmError> {
        self.alarms.save(storage, subscriber, &as_seconds(time))?;

        Ok(())
    }

    pub fn remove(&self, storage: &mut dyn Storage, subscriber: Addr) -> Result<(), AlarmError> {
        self.alarms.remove(storage, subscriber)?;

        Ok(())
    }

    pub fn alarms_selection<'b>(
        &self,
        storage: &'b dyn Storage,
        ctime: Timestamp,
    ) -> impl Iterator<Item = Result<(Addr, TimeSeconds), AlarmError>> + 'b
    where
        'a: 'b,
    {
        self.alarms
            .idx
            .alarms
            .range(
                storage,
                None,
                Some(Bound::inclusive((as_seconds(ctime), Addr::unchecked("")))),
                Order::Ascending,
            )
            .map(|res| res.map_err(AlarmError::from))
    }
}

pub struct InDelivery<'r> {
    storage: &'r mut dyn Storage,
}

impl<'r> InDelivery<'r> {
    const ALARMS: Deque<'_, TimeAlarm> = Deque::new("in_delivery");

    pub fn new(storage: &'r mut dyn Storage) -> Self {
        Self { storage }
    }

    pub fn add(&mut self, subscriber: Addr, time: TimeSeconds) -> Result<(), AlarmError> {
        Self::ALARMS
            .push_back(self.storage, &TimeAlarm { subscriber, time })
            .map_err(Into::into)
    }

    pub fn delivered(&mut self) -> Result<(), AlarmError> {
        Self::ALARMS
            .pop_front(self.storage)
            .map(|maybe_alarm: Option<TimeAlarm>| debug_assert!(maybe_alarm.is_some()))
            .map_err(Into::into)
    }

    pub fn failed(&mut self, alarms: &mut Alarms<'_>) -> Result<(), AlarmError> {
        Self::ALARMS
            .pop_front(self.storage)
            .map_err(Into::into)
            .and_then(|maybe_alarm: Option<TimeAlarm>| {
                maybe_alarm.ok_or(AlarmError::ReplyOnEmptyAlarmQueue)
            })
            .and_then(|TimeAlarm { subscriber, time }| {
                alarms.add(self.storage, subscriber, Timestamp::from_seconds(time))
            })
    }
}

#[derive(Serialize, Deserialize)]
struct TimeAlarm {
    subscriber: Addr,
    time: TimeSeconds,
}

#[cfg(test)]
pub mod tests {
    use sdk::cosmwasm_std::testing;

    use super::*;

    fn query_alarms(
        storage: &dyn Storage,
        alarms: &Alarms<'_>,
        t_sec: TimeSeconds,
    ) -> Vec<(Addr, TimeSeconds)> {
        alarms
            .alarms_selection(storage, Timestamp::from_seconds(t_sec))
            .map(Result::unwrap)
            .collect()
    }

    #[test]
    fn test_add() {
        let alarms = Alarms::new("alarms", "alarms_idx");
        let storage = &mut testing::mock_dependencies().storage;

        let t1 = Timestamp::from_seconds(1);
        let t2 = Timestamp::from_seconds(3);
        let addr1 = Addr::unchecked("addr1");
        let addr2 = Addr::unchecked("addr2");

        alarms.add(storage, addr1.clone(), t1).unwrap();

        assert_eq!(
            query_alarms(storage, &alarms, 10),
            vec![(addr1.clone(), as_seconds(t1))]
        );

        // single alarm per addr
        alarms.add(storage, addr1.clone(), t2).unwrap();

        assert_eq!(
            query_alarms(storage, &alarms, 10),
            vec![(addr1.clone(), as_seconds(t2))]
        );

        alarms.add(storage, addr2.clone(), t2).unwrap();

        assert_eq!(
            query_alarms(storage, &alarms, 10),
            vec![(addr1, as_seconds(t2)), (addr2, as_seconds(t2))]
        );
    }

    #[test]
    fn test_remove() {
        let alarms = Alarms::new("alarms", "alarms_idx");
        let storage = &mut testing::mock_dependencies().storage;

        let t1 = Timestamp::from_seconds(10);
        let t2 = Timestamp::from_seconds(20);
        let addr1 = Addr::unchecked("addr1");
        let addr2 = Addr::unchecked("addr2");

        alarms.add(storage, addr1.clone(), t1).unwrap();
        alarms.add(storage, addr2.clone(), t2).unwrap();

        assert_eq!(
            query_alarms(storage, &alarms, 30),
            vec![
                (addr1.clone(), as_seconds(t1)),
                (addr2.clone(), as_seconds(t2))
            ]
        );

        alarms.remove(storage, addr1).unwrap();
        assert_eq!(
            query_alarms(storage, &alarms, 30),
            vec![(addr2, as_seconds(t2))]
        );
    }

    #[test]
    fn test_selection() {
        let alarms = Alarms::new("alarms", "alarms_idx");
        let storage = &mut testing::mock_dependencies().storage;
        let t1 = Timestamp::from_seconds(1);
        let t2 = Timestamp::from_seconds(2);
        let t3_sec = 3;
        let t4 = Timestamp::from_seconds(4);
        let addr1 = Addr::unchecked("addr1");
        let addr2 = Addr::unchecked("addr2");
        let addr3 = Addr::unchecked("addr3");
        let addr4 = Addr::unchecked("addr4");

        // same timestamp
        alarms.add(storage, addr1.clone(), t1).unwrap();
        alarms.add(storage, addr2.clone(), t1).unwrap();
        // different timestamp
        alarms.add(storage, addr3.clone(), t2).unwrap();
        // rest
        alarms.add(storage, addr4, t4).unwrap();

        assert_eq!(
            query_alarms(storage, &alarms, t3_sec),
            vec![
                (addr1, as_seconds(t1)),
                (addr2, as_seconds(t1)),
                (addr3, as_seconds(t2))
            ]
        );
    }
}
