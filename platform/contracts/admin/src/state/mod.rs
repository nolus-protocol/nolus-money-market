use sdk::cosmwasm_std::Storage;

use crate::result::Result;

pub(crate) mod contract;
pub(crate) mod contracts;

pub(crate) fn migrate(storage: &mut dyn Storage) -> Result<()> {
    contracts::migrate_platform(storage)
}
