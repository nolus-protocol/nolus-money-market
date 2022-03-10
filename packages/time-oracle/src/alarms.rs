use cw_storage_plus::{Map, Bound};
use cosmwasm_std::{Addr, Response, StdResult, StdError, Storage, Timestamp, Order};
use std::collections::HashSet;

const ALARMS: Map<u64, HashSet<Addr>> = Map::new("alarms");

pub fn add_time_notify(storage: &mut dyn Storage, addr: Addr, time: Timestamp) -> StdResult<Response> {
    ALARMS.update::<_, StdError>(storage, time.nanos(), |records| {
        let mut records = records.unwrap_or_default();
		records.insert(addr);
		Ok(records)
    })?;

    Ok(Response::new().add_attribute("method", "add_time_notify"))
}

pub fn remove_time_notify(storage: &mut dyn Storage, addr: &Addr, time: Timestamp) -> StdResult<Response> {
    let mut records = ALARMS.may_load(storage, time.nanos())?
        // Maybe we need a package error type for this already.
        .ok_or(StdError::generic_err("trying to remove nonexistent alarm"))?;

    if !records.remove(addr) {
		return Err(StdError::generic_err("trying to remove nonexistent alarm"));
    }

    if records.is_empty() {
		ALARMS.remove(storage, time.nanos());
    }

    Ok(Response::new().add_attribute("method", "add_time_notify"))
}

fn remove_by_timestamp(storage: &mut dyn Storage, time: u64) {
	ALARMS.remove(storage, time);
}

pub trait MsgSender {
	fn send(&self, addr: &Addr, time: Timestamp) -> StdResult<Response>;
}

pub fn alarms_notify(storage: &mut dyn Storage, sender: &impl MsgSender, ctime: Timestamp) -> StdResult<Response> {

	let mut to_remove = vec![];

	let timestamps = ALARMS.range(storage, None, Some(Bound::inclusive(ctime.nanos())), Order::Ascending);
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

    Ok(Response::new().add_attribute("method", "alarms_notify"))
}


#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing;
    use std::cell::Cell;

    struct MockSender(pub Cell<u32>);

    impl MsgSender for MockSender {
    	fn send(&self, _addr: &Addr, _time: Timestamp) -> StdResult<Response> {
        	let c = self.0.get();
        	self.0.set(c+1);
    		Ok(Response::new())
    	}
    }

    #[test]
    fn test_add_time_notify() {
        let storage = &mut testing::mock_dependencies().storage;
        let t1 = Timestamp::from_seconds(1);
        let t2 = Timestamp::from_seconds(2);
        let addr1 = Addr::unchecked("addr1");
        let addr2 = Addr::unchecked("addr2");
        let addr3 = Addr::unchecked("addr3");

		add_time_notify(storage, addr1.clone(), t1).expect("can't set alarms");
		add_time_notify(storage, addr2.clone(), t1).expect("can't set alarms");
		add_time_notify(storage, addr3.clone(), t2).expect("can't set alarms");

		let data = ALARMS.load(storage, t1.nanos()).expect("can't load alarms");

		let reference = HashSet::from([addr1.clone(), addr2.clone()]);
		assert_eq!(data, reference);
    }

    #[test]
    fn test_remove_time_notify() {
        let storage = &mut testing::mock_dependencies().storage;
        let t1 = Timestamp::from_seconds(1);
        let t2 = Timestamp::from_seconds(2);
        let addr1 = Addr::unchecked("addr1");
        let addr2 = Addr::unchecked("addr2");
        let addr3 = Addr::unchecked("addr3");

		add_time_notify(storage, addr1.clone(), t1).expect("can't set alarms");
		add_time_notify(storage, addr2.clone(), t1).expect("can't set alarms");
		add_time_notify(storage, addr3.clone(), t2).expect("can't set alarms");

		remove_time_notify(storage, &addr1, t1).expect("can't remove alarm");

		let data = ALARMS.load(storage, t1.nanos()).expect("can't load alarms");
		let reference = HashSet::from([addr2.clone()]);
		assert_eq!(data, reference);

		remove_time_notify(storage, &addr3, t2).expect("can't remove alarm");

		let data = ALARMS.may_load(storage, t1.nanos()).expect("can't load alarms");
		assert_eq!(data, None);
    }


}
