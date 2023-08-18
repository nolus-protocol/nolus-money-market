#![cfg(feature = "migration")]

use serde::{Deserialize, Serialize, Serializer};

use finance::percent::bound::BoundToHundredPercent;
use sdk::{
    cosmwasm_std::{Storage, Uint64},
    cw_storage_plus::Item,
};

use crate::{borrow::InterestRate, error::Result as ContractResult, state::Config};

#[derive(Deserialize)]
struct OldConfig {
    lpn_ticker: String,
    lease_code_id: Uint64,
    borrow_rate: InterestRate,
}

impl OldConfig {
    const STORAGE: Item<'static, Self> = Item::new("config");
}

impl Serialize for OldConfig {
    fn serialize<S>(&self, _: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        unimplemented!("Required by `cw_storage_plus::Item::load`'s trait bounds.")
    }
}

pub fn migrate(
    storage: &mut dyn Storage,
    min_utilization: BoundToHundredPercent,
) -> ContractResult<()> {
    OldConfig::STORAGE
        .load(storage)
        .map(
            |OldConfig {
                 lpn_ticker,
                 lease_code_id,
                 borrow_rate,
             }| {
                Config::new(lpn_ticker, lease_code_id, borrow_rate, min_utilization)
            },
        )
        .map_err(Into::into)
        .and_then(|config: Config| config.store(storage))
}
