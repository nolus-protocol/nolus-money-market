use cosmwasm_std::{StdResult, Storage, Uint64};
use cw_storage_plus::Item;
use finance::{currency::Currency, percent::Percent, price::Price};
use schemars::JsonSchema;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::nlpn::NLpn;

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, JsonSchema)]
pub struct Config {
    pub currency: String,
    pub lease_code_id: Uint64,
    pub base_interest_rate: Percent,
    pub utilization_optimal: Percent,
    pub addon_optimal_interest_rate: Percent,
}

impl Config {
    const STORAGE: Item<'static, Self> = Item::new("config");

    pub fn new(currency: String, lease_code_id: Uint64) -> Self {
        Config {
            currency,
            lease_code_id,
            base_interest_rate: Percent::from_percent(7),
            utilization_optimal: Percent::from_percent(70),
            addon_optimal_interest_rate: Percent::from_percent(2),
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

    pub fn initial_derivative_price<LPN>() -> Price<NLpn, LPN>
    where
        LPN: Currency + Serialize + DeserializeOwned,
    {
        Price::identity()
    }
}
