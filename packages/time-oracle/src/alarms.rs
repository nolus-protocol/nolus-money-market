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

pub fn remove(storage: &mut dyn Storage, addr: &Addr, time: Timestamp) -> StdResult<()> {
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

    Ok(())
}

fn remove_by_timestamp(storage: &mut dyn Storage, time: u64) {
    ALARMS.remove(storage, time);
}

pub fn notify(
    storage: &mut dyn Storage,
    ctime: Timestamp,
) -> StdResult<Vec<Addr>> {
    let mut to_remove = vec![];
    let mut collector = vec![];


    let timestamps = ALARMS.range(
        storage,
        None,
        Some(Bound::inclusive(ctime.nanos())),
        Order::Ascending,
    );
    for alarms in timestamps {
        let (timestamp, adresses) = alarms?;
		collector.extend(adresses);
        to_remove.push(timestamp);
    }

    for t in to_remove {
        remove_by_timestamp(storage, t);
    }

    Ok(collector)
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use cosmwasm_std::testing;

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

        // same timestamp
        add(storage, addr1.clone(), t1).expect("can't set alarms");
        add(storage, addr2.clone(), t1).expect("can't set alarms");
        // other timestamp
        add(storage, addr3.clone(), t2).expect("can't set alarms");
        // rest
        add(storage, addr4.clone(), t4).expect("can't set alarms");

        let mut res = notify(storage, t1).expect("can't notify alarms");
        res.sort();
        assert_eq!(res, [addr1, addr2]);

        let res = notify(storage, t3).expect("can't notify alarms");
        assert_eq!(res, [addr3]);
    }
}
