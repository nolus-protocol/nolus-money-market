use std::ops::{Deref, DerefMut};

use sdk::{
    cosmwasm_std::{Addr, Order, Storage, Timestamp},
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

type IndexedMap = CwIndexedMap<Addr, TimeSeconds, AlarmIndexes>;

pub struct Alarms<'storage, S>
where
    S: Deref<Target = dyn Storage + 'storage>,
{
    storage: S,
    alarms: IndexedMap,
    in_delivery: Deque<Addr>,
}

impl<'storage, S> Alarms<'storage, S>
where
    S: Deref<Target = dyn Storage + 'storage>,
{
    pub fn new(
        storage: S,
        namespace_alarms: &'static str,
        namespace_index: &'static str,
        namespace_in_delivery: &'static str,
    ) -> Self {
        Self {
            storage,
            alarms: indexed_map(namespace_alarms, namespace_index),
            in_delivery: Deque::new(namespace_in_delivery),
        }
    }

    pub fn alarms_selection<'this>(
        &'this self,
        ctime: Timestamp,
    ) -> impl Iterator<Item = Result<Addr, AlarmError>> + use<'this, S> {
        self.alarms
            .idx
            .alarms
            .range(
                self.storage.deref(),
                None,
                Some(Bound::inclusive((as_seconds(ctime), Addr::unchecked("")))),
                Order::Ascending,
            )
            .map(|res| {
                res.map(|(subscriber, _): (Addr, TimeSeconds)| subscriber)
                    .map_err(AlarmError::from)
            })
    }
}

impl<'storage, S> Alarms<'storage, S>
where
    S: Deref<Target = dyn Storage + 'storage> + DerefMut,
{
    pub fn add(&mut self, subscriber: Addr, time: Timestamp) -> Result<(), AlarmError> {
        self.add_internal(subscriber, as_seconds(time))
    }

    pub fn ensure_no_in_delivery(&mut self) -> Result<&mut Self, AlarmError> {
        self.in_delivery
            .is_empty(self.storage.deref_mut())?
            .then_some(self)
            .ok_or_else(|| {
                AlarmError::NonEmptyAlarmsInDeliveryQueue(String::from("Assertion requested"))
            })
    }

    pub fn out_for_delivery(&mut self, subscriber: Addr) -> Result<(), AlarmError> {
        self.alarms
            .remove(self.storage.deref_mut(), subscriber.clone())?;

        self.in_delivery
            .push_back(self.storage.deref_mut(), &subscriber)
            .map_err(Into::into)
    }

    pub fn last_delivered(&mut self) -> Result<(), AlarmError> {
        self.in_delivery
            .pop_front(self.storage.deref_mut())
            .map_err(Into::into)
            .and_then(|maybe_alarm: Option<Addr>| {
                if maybe_alarm.is_some() {
                    Ok(())
                } else {
                    Err(AlarmError::EmptyAlarmsInDeliveryQueue(String::from(
                        "Received success reply status",
                    )))
                }
            })
    }

    pub fn last_failed(&mut self, now: Timestamp) -> Result<(), AlarmError> {
        self.in_delivery
            .pop_front(self.storage.deref_mut())
            .map_err(Into::into)
            .and_then(|maybe_alarm: Option<Addr>| maybe_alarm.ok_or_else(|| AlarmError::EmptyAlarmsInDeliveryQueue(
                String::from("Received failure reply status"))
            ))
            .and_then(|subscriber: Addr| self.add_internal(subscriber, as_seconds(now) - /* Minus one second, to ensure it can be run within the same block */ 1))
    }

    fn add_internal(&mut self, subscriber: Addr, time: TimeSeconds) -> Result<(), AlarmError> {
        self.alarms
            .save(self.storage.deref_mut(), subscriber, &time)
            .map_err(Into::into)
    }
}

#[cfg(test)]
pub mod tests {
    use sdk::cosmwasm_std::testing::MockStorage;

    use super::*;

    fn alarms<'storage>(
        storage: &'storage mut (dyn Storage + 'storage),
    ) -> Alarms<'storage, &'storage mut (dyn Storage + 'storage)> {
        Alarms::new(storage, "alarms", "alarms_idx", "in_delivery")
    }

    #[allow(clippy::needless_lifetimes)] // cannot rely on eliding lifetimes due to a known limitattion, look at the clippy lint description
    fn query_alarms<'r, S>(alarms: &Alarms<'r, S>, t_sec: TimeSeconds) -> Vec<Addr>
    where
        S: Deref<Target = dyn Storage + 'r>,
    {
        alarms
            .alarms_selection(Timestamp::from_seconds(t_sec))
            .map(Result::unwrap)
            .collect()
    }

    #[test]
    fn test_add() {
        let mut storage = MockStorage::new();
        let mut alarms = alarms(&mut storage);

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
        let mut storage = MockStorage::new();
        let mut alarms = alarms(&mut storage);

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
