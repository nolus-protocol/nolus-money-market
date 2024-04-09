use std::collections::BTreeMap;

use sdk::cosmwasm_std::Storage;

use crate::{contracts::Dex, result::Result};

pub(crate) mod contract;
pub(crate) mod contracts;

pub(crate) fn migrate(storage: &mut dyn Storage, dexes: BTreeMap<String, Dex>) -> Result<()> {
    contracts::migrate_protocols(storage, dexes)
}
