use cosmwasm_std::{Decimal, StdResult, Storage, Uint64};
use cw_storage_plus::Item;
use finance::percent::Percent;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub currency: String,
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
            currency: denom,
            lease_code_id,
            base_interest_rate: Percent::from_percent(7),
            utilization_optimal: Percent::from_percent(70),
            addon_optimal_interest_rate: Percent::from_percent(2),
            initial_derivative_price: Decimal::one(),
        }
    }

    pub fn store(&self, storage: &mut dyn Storage) -> StdResult<()> {
        Self::STORAGE.save(storage, self)
    }

    pub fn load(storage: &dyn Storage) -> StdResult<Self> {
        Self::STORAGE.load(storage)
    }

    pub fn update(
        &mut self,
        storage: &mut dyn Storage,
        base_interest_rate: Percent,
        utilization_optimal: Percent,
        addon_optimal_interest_rate: Percent,
    ) -> StdResult<()> {
        self.base_interest_rate = base_interest_rate;
        self.utilization_optimal = utilization_optimal;
        self.addon_optimal_interest_rate = addon_optimal_interest_rate;

        self.store(storage)
    }
}
