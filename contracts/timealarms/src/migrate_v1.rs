use cosmwasm_std::{Addr, Storage, Timestamp};
use time_oracle::Alarms;

use crate::ContractError;

const PROFIT_ADDR: &str = "nolus1mf6ptkssddfmxvhdx0ech0k03ktp6kf9yk59renau2gvht3nq2gqkxgywu";
const REWARDS_DISPATCHER_ADDR: &str =
    "nolus1tqwwyth34550lg2437m05mjnjp8w7h5ka7m70jtzpxn4uh2ktsmqtctwnn";
const ALARM_ON: Timestamp = Timestamp::from_seconds(1701263948);

pub fn migrate_dev(storage: &mut dyn Storage, alarms: &Alarms) -> Result<(), ContractError> {
    alarms
        .add(storage, Addr::unchecked(PROFIT_ADDR), ALARM_ON)
        .and_then(|()| alarms.add(storage, Addr::unchecked(REWARDS_DISPATCHER_ADDR), ALARM_ON))
        .map_err(Into::into)
}
