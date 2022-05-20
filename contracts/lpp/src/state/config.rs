use serde::{Serialize, Deserialize};
use schemars::JsonSchema;
use cosmwasm_std::{Uint64, Decimal, Storage, StdResult};
use cw_storage_plus::Item;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub denom: String,
    pub lease_code_id: Uint64,
    pub base_interest_rate: Decimal,
    pub utilization_optimal: Decimal,
    pub addon_optimal_interest_rate: Decimal,
}

impl Config {
    const STORAGE: Item<'static, Self> = Item::new("config");

    pub fn new(denom: String, lease_code_id: Uint64) -> Self {
        Config {
            denom,
            lease_code_id,
            base_interest_rate: Decimal::percent(7),
            utilization_optimal: Decimal::percent(70),
            addon_optimal_interest_rate: Decimal::percent(2),
        }
    }

    pub fn store(self, storage: &mut dyn Storage) -> StdResult<()> {
        Self::STORAGE.save(storage, &self)
    }

    pub fn load(storage: &dyn Storage) -> StdResult<Self> {
        Self::STORAGE.load(storage)
    }


}
