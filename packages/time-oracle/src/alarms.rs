use cosmwasm_std::{Addr, Order, Response, StdError, StdResult, Storage, Timestamp};
use cw_storage_plus::{Bound, Map};
use std::collections::HashSet;

const ALARMS: Map<u64, HashSet<Addr>> = Map::new("alarms");

pub fn add(storage: &mut dyn Storage, addr: Addr, time: Timestamp) -> StdResult<Response> {
    ALARMS.update::<_, StdError>(storage, time.nanos(), |records| {
        let mut records = records.unwrap_or_default();
        records.insert(addr);
        Ok(records)
    })?;

    Ok(Response::new().add_attribute("method", "add"))
}

pub fn remove(storage: &mut dyn Storage, addr: &Addr, time: Timestamp) -> StdResult<Response> {
    let mut is_empty = false;

    ALARMS.update::<_, StdError>(storage, time.nanos(), |records| {
        if let Some(mut records) = records {
            if !records.remove(addr) {
                return Err(StdError::generic_err("can't remove alarm"));
            }
            is_empty = records.is_empty();
            Ok(records)
        } else {
            Err(StdError::generic_err("can't remove alarm"))
        }
    })?;

    if is_empty {
        ALARMS.remove(storage, time.nanos());
    }

    Ok(Response::new().add_attribute("method", "add"))
}

fn remove_by_timestamp(storage: &mut dyn Storage, time: u64) {
    ALARMS.remove(storage, time);
}

pub trait MsgSender {
    fn send(&self, addr: &Addr, time: Timestamp) -> StdResult<Response>;
}

pub fn notify(
    storage: &mut dyn Storage,
    sender: &impl MsgSender,
    ctime: Timestamp,
) -> StdResult<Response> {
    let mut to_remove = vec![];

    let timestamps = ALARMS.range(
        storage,
        None,
        Some(Bound::inclusive(ctime.nanos())),
        Order::Ascending,
    );
    for alarms in timestamps {
        let (timestamp, adresses) = alarms?;
        for address in adresses {
            sender.send(&address, ctime)?;
        }
        to_remove.push(timestamp);
    }

    for t in to_remove {
        remove_by_timestamp(storage, t);
    }

    Ok(Response::new().add_attribute("method", "notify"))
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use cosmwasm_std::testing;
    use std::cell::Cell;

    pub struct MockSender(pub Cell<u32>);

    impl MockSender {
        pub fn new() -> Self {
            MockSender(Cell::new(0))
        }
    }

    impl MsgSender for MockSender {
        // count send messages
        fn send(&self, _addr: &Addr, _time: Timestamp) -> StdResult<Response> {
            let c = self.0.get();
            self.0.set(c + 1);
            Ok(Response::new())
        }
    }

    #[test]
    fn test_add() {
        let storage = &mut testing::mock_dependencies().storage;
        let t1 = Timestamp::from_seconds(1);
        let t2 = Timestamp::from_seconds(2);
        let addr1 = Addr::unchecked("addr1");
        let addr2 = Addr::unchecked("addr2");
        let addr3 = Addr::unchecked("addr3");

        add(storage, addr1.clone(), t1).expect("can't set alarms");
        add(storage, addr2.clone(), t1).expect("can't set alarms");
        add(storage, addr3.clone(), t2).expect("can't set alarms");

        let data = ALARMS.load(storage, t1.nanos()).expect("can't load alarms");

        let reference = HashSet::from([addr1.clone(), addr2.clone()]);
        assert_eq!(data, reference);
    }

    #[test]
    fn test_remove() {
        let storage = &mut testing::mock_dependencies().storage;
        let t1 = Timestamp::from_seconds(1);
        let t2 = Timestamp::from_seconds(2);
        let addr1 = Addr::unchecked("addr1");
        let addr2 = Addr::unchecked("addr2");
        let addr3 = Addr::unchecked("addr3");
        let addr4 = Addr::unchecked("addr4");

        add(storage, addr1.clone(), t1).expect("can't set alarms");
        add(storage, addr2.clone(), t1).expect("can't set alarms");
        add(storage, addr3.clone(), t2).expect("can't set alarms");

        remove(storage, &addr1, t1).expect("can't remove alarm");

        // remove from nonempty timestamp collection
        let data = ALARMS.load(storage, t1.nanos()).expect("can't load alarms");
        let reference = HashSet::from([addr2.clone()]);
        assert_eq!(data, reference);

        // remove with timestamp collection cleanup
        remove(storage, &addr3, t2).expect("can't remove alarm");
        let data = ALARMS
            .may_load(storage, t2.nanos())
            .expect("can't load alarms");
        assert_eq!(data, None);

        // try to remove unexistent alarm from collection
        let err = remove(storage, &addr4, t1).map_err(|_| ());
        assert_eq!(err, Err(()));

        // try to remove alarm from unexistent timestamp
        let err = remove(storage, &addr4, t2).map_err(|_| ());
        assert_eq!(err, Err(()));
    }

    #[test]
    fn test_notify() {
        let storage = &mut testing::mock_dependencies().storage;
        let t1 = Timestamp::from_seconds(1);
        let t2 = Timestamp::from_seconds(2);
        let t3 = Timestamp::from_seconds(3);
        let t4 = Timestamp::from_seconds(4);
        let addr1 = Addr::unchecked("addr1");
        let addr2 = Addr::unchecked("addr2");
        let addr3 = Addr::unchecked("addr3");
        let addr4 = Addr::unchecked("addr4");
        let sender = MockSender::new();

        // same timestamp
        add(storage, addr1.clone(), t1).expect("can't set alarms");
        add(storage, addr2.clone(), t1).expect("can't set alarms");
        // other timestamp
        add(storage, addr3.clone(), t2).expect("can't set alarms");
        // rest
        add(storage, addr4.clone(), t4).expect("can't set alarms");

        notify(storage, &sender, t1).expect("can't notify alarms");
        assert_eq!(sender.0.get(), 2);

        notify(storage, &sender, t3).expect("can't notify alarms");
        assert_eq!(sender.0.get(), 3);
    }
}
