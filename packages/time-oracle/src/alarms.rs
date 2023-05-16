use std::iter;

use sdk::{
    cosmwasm_std::{Addr, Order, StdResult as CwResult, Storage, Timestamp},
    cw_storage_plus::{Bound, Deque, Index, IndexList, IndexedMap as CwIndexedMap, MultiIndex},
};

use crate::AlarmError;

type TimeSeconds = u64;

fn as_seconds(from: Timestamp) -> TimeSeconds {
    from.seconds()
}

struct AlarmIndexes {
    alarms: MultiIndex<'static, TimeSeconds, TimeSeconds, Addr>,
}

impl IndexList<TimeSeconds> for AlarmIndexes {
    fn get_indexes(&self) -> Box<dyn Iterator<Item = &'_ dyn Index<TimeSeconds>> + '_> {
        let v: Vec<&dyn Index<TimeSeconds>> = vec![&self.alarms];

        Box::new(v.into_iter())
    }
}

fn indexed_map(namespace_alarms: &'static str, namespace_index: &'static str) -> IndexedMap {
    let indexes = AlarmIndexes {
        alarms: MultiIndex::new(|_, d| *d, namespace_alarms, namespace_index),
    };

    IndexedMap::new(namespace_alarms, indexes)
}

type IndexedMap = CwIndexedMap<'static, Addr, TimeSeconds, AlarmIndexes>;

const ALARMS_IN_DELIVERY: Deque<'static, Addr> = Deque::new("in_delivery");

type AlarmsSelectionIterator<'storage> = iter::Map<
    Box<dyn Iterator<Item = CwResult<(Addr, TimeSeconds)>> + 'storage>,
    fn(CwResult<(Addr, TimeSeconds)>) -> Result<Addr, AlarmError>,
>;

pub trait AlarmsSelection {
    fn alarms_selection(&self, ctime: Timestamp) -> AlarmsSelectionIterator<'_>;
}

pub struct Alarms<'storage> {
    storage: &'storage dyn Storage,
    alarms: IndexedMap,
}

impl<'storage> Alarms<'storage> {
    pub fn new(
        storage: &'storage dyn Storage,
        namespace_alarms: &'static str,
        namespace_index: &'static str,
    ) -> Self {
        Self {
            storage,
            alarms: indexed_map(namespace_alarms, namespace_index),
        }
    }
}

impl<'storage> AlarmsSelection for Alarms<'storage> {
    fn alarms_selection(&self, ctime: Timestamp) -> AlarmsSelectionIterator<'_> {
        alarms_selection(self.storage, &self.alarms, as_seconds(ctime))
    }
}

pub struct AlarmsMut<'storage> {
    storage: &'storage mut dyn Storage,
    alarms: IndexedMap,
}

impl<'storage> AlarmsMut<'storage> {
    pub fn new(
        storage: &'storage mut dyn Storage,
        namespace_alarms: &'static str,
        namespace_index: &'static str,
    ) -> Self {
        Self {
            storage,
            alarms: indexed_map(namespace_alarms, namespace_index),
        }
    }

    pub fn add(&mut self, subscriber: Addr, time: Timestamp) -> Result<(), AlarmError> {
        self.add_internal(subscriber, as_seconds(time))
    }

    pub fn ensure_no_in_delivery(&mut self) -> Result<&mut Self, AlarmError> {
        ALARMS_IN_DELIVERY
            .is_empty(self.storage)?
            .then_some(self)
            .ok_or(AlarmError::NonEmptyAlarmQueue)
    }

    pub fn out_for_delivery(&mut self, subscriber: Addr) -> Result<(), AlarmError> {
        self.alarms.remove(self.storage, subscriber.clone())?;

        ALARMS_IN_DELIVERY
            .push_back(self.storage, &subscriber)
            .map_err(Into::into)
    }

    pub fn last_delivered(&mut self) -> Result<(), AlarmError> {
        ALARMS_IN_DELIVERY
            .pop_front(self.storage)
            .map(|maybe_alarm: Option<Addr>| debug_assert!(maybe_alarm.is_some()))
            .map_err(Into::into)
    }

    pub fn last_failed(&mut self, now: Timestamp) -> Result<(), AlarmError> {
        ALARMS_IN_DELIVERY.pop_front(self.storage).map_err(Into::into).and_then(|maybe_alarm: Option<Addr>| {
            maybe_alarm.ok_or(AlarmError::ReplyOnEmptyAlarmQueue)
        }).and_then(|subscriber: Addr| self.add_internal(subscriber, as_seconds(now) - /* Minus one second, to ensure it can be run within the same block */ 1))
    }

    fn add_internal(&mut self, subscriber: Addr, time: TimeSeconds) -> Result<(), AlarmError> {
        self.alarms
            .save(self.storage, subscriber, &time)
            .map_err(Into::into)
    }
}

impl<'storage> AlarmsSelection for AlarmsMut<'storage> {
    fn alarms_selection(&self, ctime: Timestamp) -> AlarmsSelectionIterator<'_> {
        alarms_selection(self.storage, &self.alarms, as_seconds(ctime))
    }
}

fn alarms_selection<'storage>(
    storage: &'storage dyn Storage,
    alarms: &IndexedMap,
    time: TimeSeconds,
) -> AlarmsSelectionIterator<'storage> {
    alarms
        .idx
        .alarms
        .range(
            storage,
            None,
            Some(Bound::inclusive((time, Addr::unchecked("")))),
            Order::Ascending,
        )
        .map(|res| {
            res.map(|(subscriber, _): (Addr, TimeSeconds)| subscriber)
                .map_err(AlarmError::from)
        })
}

#[cfg(test)]
pub mod tests {
    use sdk::cosmwasm_std::testing;

    use super::*;

    fn query_alarms<AS>(alarms: &AS, t_sec: TimeSeconds) -> Vec<Addr>
    where
        AS: AlarmsSelection,
    {
        alarms
            .alarms_selection(Timestamp::from_seconds(t_sec))
            .map(Result::unwrap)
            .collect()
    }

    #[test]
    fn test_add() {
        let storage = &mut testing::mock_dependencies().storage;
        let mut alarms = AlarmsMut::new(storage, "alarms", "alarms_idx");

        let t1 = Timestamp::from_seconds(1);
        let t2 = Timestamp::from_seconds(3);
        let addr1 = Addr::unchecked("addr1");
        let addr2 = Addr::unchecked("addr2");

        alarms.add(addr1.clone(), t1).unwrap();

        assert_eq!(query_alarms(&alarms, 10), vec![addr1.clone()]);

        // single alarm per addr
        alarms.add(addr1.clone(), t2).unwrap();

        assert_eq!(query_alarms(&alarms, 10), vec![addr1.clone()]);

        alarms.add(addr2.clone(), t2).unwrap();

        assert_eq!(query_alarms(&alarms, 10), vec![addr1, addr2]);
    }

    #[test]
    fn test_selection() {
        let storage = &mut testing::mock_dependencies().storage;
        let mut alarms = AlarmsMut::new(storage, "alarms", "alarms_idx");
        let t1 = Timestamp::from_seconds(1);
        let t2 = Timestamp::from_seconds(2);
        let t3_sec = 3;
        let t4 = Timestamp::from_seconds(4);
        let addr1 = Addr::unchecked("addr1");
        let addr2 = Addr::unchecked("addr2");
        let addr3 = Addr::unchecked("addr3");
        let addr4 = Addr::unchecked("addr4");

        // same timestamp
        alarms.add(addr1.clone(), t1).unwrap();
        alarms.add(addr2.clone(), t1).unwrap();
        // different timestamp
        alarms.add(addr3.clone(), t2).unwrap();
        // rest
        alarms.add(addr4, t4).unwrap();

        assert_eq!(query_alarms(&alarms, t3_sec), vec![addr1, addr2, addr3]);
    }
}
