use cosmwasm_std::{Addr, Order, StdError, StdResult, Storage, Timestamp};
use cw_storage_plus::{Bound, Map};
use std::collections::HashSet;

pub struct Alarms<'a>(Map<'a, u64, HashSet<Addr>>);

impl<'a> Alarms<'a> {
    pub fn new(namespace: &'a str) -> Self {
        Alarms(Map::new(namespace))
    }

    pub fn add(&self, storage: &mut dyn Storage, addr: Addr, time: Timestamp) -> StdResult<()> {
        self.0
            .update::<_, StdError>(storage, time.seconds(), |records| {
                let mut records = records.unwrap_or_default();
                records.insert(addr);
                Ok(records)
            })?;

        Ok(())
    }

    pub fn remove(&self, storage: &mut dyn Storage, addr: &Addr, time: Timestamp) -> StdResult<()> {
        let mut is_empty = false;

        self.0
            .update::<_, StdError>(storage, time.seconds(), |records| {
                if let Some(mut records) = records {
                    if !records.remove(addr) {
                        return Err(StdError::generic_err("Unknown alarm recipient"));
                    }
                    is_empty = records.is_empty();
                    Ok(records)
                } else {
                    Err(StdError::generic_err("Unknown alarm timestamp"))
                }
            })?;

        if is_empty {
            self.0.remove(storage, time.seconds());
        }

        Ok(())
    }

    pub fn notify(
        &self,
        storage: &mut dyn Storage,
        dispatcher: &mut impl AlarmDispatcher,
        ctime: Timestamp,
    ) -> StdResult<()> {
        let mut to_remove = vec![];

        let timestamps = self.0.range(
            storage,
            None,
            Some(Bound::inclusive(ctime.seconds())),
            Order::Ascending,
        );
        for alarms in timestamps {
            let (timestamp, adresses) = alarms?;
            for addr in adresses {
                dispatcher.send_to(addr);
            }
            to_remove.push(timestamp);
        }

        for t in to_remove {
            self.remove_by_timestamp(storage, t);
        }

        Ok(())
    }

    fn remove_by_timestamp(&self, storage: &mut dyn Storage, time: u64) {
        self.0.remove(storage, time);
    }
}

pub trait AlarmDispatcher {
    fn send_to(&mut self, addr: Addr);
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use cosmwasm_std::testing;

    #[derive(Default)]
    struct MockAlarmDispatcher(pub Vec<Addr>);

    impl AlarmDispatcher for MockAlarmDispatcher {
        fn send_to(&mut self, addr: Addr) {
            self.0.push(addr);
        }
    }

    #[test]
    fn test_add() {
        let alarms = Alarms::new("alarms");
        let storage = &mut testing::mock_dependencies().storage;
        let mut dispatcher = MockAlarmDispatcher::default();

        let t1 = Timestamp::from_seconds(1);
        let t2 = Timestamp::from_seconds(2);
        let addr1 = Addr::unchecked("addr1");
        let addr2 = Addr::unchecked("addr2");
        let addr3 = Addr::unchecked("addr3");

        assert_eq!(alarms.add(storage, addr1.clone(), t1), Ok(()));
        // same timestamp
        assert_eq!(alarms.add(storage, addr2.clone(), t1), Ok(()));
        // different timestamp
        assert_eq!(alarms.add(storage, addr3.clone(), t2), Ok(()));

        assert_eq!(alarms.notify(storage, &mut dispatcher, t2), Ok(()));
        dispatcher.0.sort();
        assert_eq!(dispatcher.0, [addr1, addr2, addr3]);
    }

    #[test]
    fn test_remove() {
        let alarms = Alarms::new("alarms");
        let storage = &mut testing::mock_dependencies().storage;
        let mut dispatcher = MockAlarmDispatcher::default();
        let t1 = Timestamp::from_seconds(1);
        let t2 = Timestamp::from_seconds(2);
        let addr1 = Addr::unchecked("addr1");
        let addr2 = Addr::unchecked("addr2");
        let addr3 = Addr::unchecked("addr3");
        let addr4 = Addr::unchecked("addr4");

        // same time stamp
        alarms
            .add(storage, addr1.clone(), t1)
            .expect("can't set alarms");
        alarms
            .add(storage, addr2.clone(), t1)
            .expect("can't set alarms");
        // different timestamp
        alarms
            .add(storage, addr3.clone(), t2)
            .expect("can't set alarms");

        assert_eq!(alarms.remove(storage, &addr1, t1), Ok(()));

        // remove with timestamp collection cleanup
        assert_eq!(alarms.remove(storage, &addr3, t2), Ok(()));

        // unknown alarm recipient
        let err = alarms.remove(storage, &addr4, t1).map_err(|_| ());
        assert_eq!(err, Err(()));

        // unknown alarm timestamp
        let err = alarms.remove(storage, &addr4, t2).map_err(|_| ());
        assert_eq!(err, Err(()));

        assert_eq!(alarms.notify(storage, &mut dispatcher, t2), Ok(()));
        assert_eq!(dispatcher.0, [addr2]);
    }

    #[test]
    fn test_notify() {
        let alarms = Alarms::new("alarms");
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
        alarms
            .add(storage, addr1.clone(), t1)
            .expect("can't set alarms");
        alarms
            .add(storage, addr2.clone(), t1)
            .expect("can't set alarms");
        // different timestamp
        alarms
            .add(storage, addr3.clone(), t2)
            .expect("can't set alarms");
        // rest
        alarms.add(storage, addr4, t4).expect("can't set alarms");

        assert_eq!(alarms.notify(storage, &mut dispatcher, t1), Ok(()));
        dispatcher.0.sort();
        assert_eq!(dispatcher.0, [addr1, addr2]);

        let mut dispatcher = MockAlarmDispatcher::default();
        assert_eq!(alarms.notify(storage, &mut dispatcher, t3), Ok(()));
        assert_eq!(dispatcher.0, [addr3]);
    }
}
