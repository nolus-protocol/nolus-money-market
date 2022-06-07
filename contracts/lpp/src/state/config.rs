use serde::{Serialize, Deserialize};
use schemars::JsonSchema;
use cosmwasm_std::{Uint64, Storage, StdResult, Decimal};
use cw_storage_plus::Item;
use finance::percent::Percent;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub denom: String,
    pub lease_code_id: Uint64,
    pub base_interest_rate: Percent,
    pub utilization_optimal: Percent,
    pub addon_optimal_interest_rate: Percent,
    pub initial_derivative_price: Decimal,
}

impl Config {
    const STORAGE: Item<'static, Self> = Item::new("config");

    pub fn new(denom: String, lease_code_id: Uint64) -> Self {
        Config {
            denom,
            lease_code_id,
            base_interest_rate: Percent::from_percent(7),
            utilization_optimal: Percent::from_percent(70),
            addon_optimal_interest_rate: Percent::from_percent(2),
            initial_derivative_price: Decimal::one(),
        }
    }

    pub fn store(self, storage: &mut dyn Storage) -> StdResult<()> {
        Self::STORAGE.save(storage, &self)
    }

    pub fn load(storage: &dyn Storage) -> StdResult<Self> {
        Self::STORAGE.load(storage)
    }


}
