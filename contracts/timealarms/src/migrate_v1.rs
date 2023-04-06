use sdk::cosmwasm_std::Storage;
use time_oracle::migrate_v1::AlarmsOld;

use crate::ContractError;

pub fn migrate(storage: &mut dyn Storage) -> Result<(), ContractError> {
    AlarmsOld::new("alarms", "alarms_idx", "alarms_next_id").migrate(
        storage,
        "alarms_map",
        "alarms_index",
    )?;

    Ok(())
}
