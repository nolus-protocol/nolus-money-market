use serde::{Deserialize, Serialize};

use currency::SymbolOwned;
use marketprice::config::Config as PriceConfig;
use sdk::{
    cosmwasm_std::{StdResult, Storage},
    cw_storage_plus::Item,
    schemars::{self, JsonSchema},
};

use crate::ContractError;

#[derive(Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
#[cfg_attr(any(test, feature = "testing"), derive(Debug, Clone))]
pub struct Config {
    pub base_asset: SymbolOwned,
    pub price_config: PriceConfig,
}

impl Config {
    const STORAGE: Item<'static, Self> = Item::new("config");

    pub fn new(base_asset: SymbolOwned, price_config: PriceConfig) -> Self {
        Self {
            base_asset,
            price_config,
        }
    }

    pub fn store(self, storage: &mut dyn Storage) -> StdResult<()> {
        Self::STORAGE.save(storage, &self)
    }

    pub fn load(storage: &dyn Storage) -> StdResult<Self> {
        Self::STORAGE.load(storage)
    }

    pub fn update(
        storage: &mut dyn Storage,
        price_config: PriceConfig,
    ) -> Result<(), ContractError> {
        Self::STORAGE.update(storage, |mut c| -> StdResult<_> {
            c.price_config = price_config;
            Ok(c)
        })?;
        Ok(())
    }
}
