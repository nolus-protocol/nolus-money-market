#[cfg(feature = "migrate")]
use std::collections::BTreeMap;

#[cfg(feature = "migrate")]
use sdk::cosmwasm_std::{Api, QuerierWrapper, Storage};

#[cfg(feature = "migrate")]
use crate::result::Result;

pub(crate) mod contract;
pub(crate) mod contracts;

#[cfg(feature = "migrate")]
pub(crate) fn migrate(
    storage: &mut dyn Storage,
    api: &dyn Api,
    querier: QuerierWrapper<'_>,
    reserve_contracts: BTreeMap<String, String>,
) -> Result<()> {
    contracts::migrate_protocols(storage, api, querier, reserve_contracts)
}
