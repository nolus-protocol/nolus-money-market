use std::mem;

use marketprice::config::Config as PriceConfig;
use sdk::{
    cosmwasm_std::{StdResult, Storage},
    cw_storage_plus::Item,
};

use crate::{api::Config, error::Result as ContractResult, ContractError};

impl Config {
    const STORAGE: Item<'static, Self> = Item::new("config");

    pub fn new(price_config: PriceConfig) -> Self {
        Self { price_config }
    }

    pub fn store(self, storage: &mut dyn Storage) -> ContractResult<()> {
        Self::STORAGE
            .save(storage, &self)
            .map_err(ContractError::StoreConfig)
    }

    pub fn load(storage: &dyn Storage) -> ContractResult<Self> {
        Self::STORAGE
            .load(storage)
            .map_err(ContractError::LoadConfig)
    }

    pub fn update(storage: &mut dyn Storage, price_config: PriceConfig) -> ContractResult<()> {
        Self::STORAGE
            .update(storage, |mut c| -> StdResult<_> {
                c.price_config = price_config;
                Ok(c)
            })
            .map(mem::drop)
            .map_err(ContractError::UpdateConfig)
    }

    pub fn migrate(storage: &mut dyn Storage) -> ContractResult<()> {
        use currency::SymbolOwned;
        use serde::{Deserialize, Serialize, Serializer};

        const STORAGE_V0: Item<'static, OldConfig> = Item::new("config");

        #[derive(Deserialize)]
        #[serde(deny_unknown_fields, rename_all = "snake_case")]
        struct OldConfig {
            #[allow(dead_code)]
            base_asset: SymbolOwned,
            price_config: PriceConfig,
        }
        impl Serialize for OldConfig {
            fn serialize<S>(&self, _serializer: S) -> Result<S::Ok, S::Error>
            where
                S: Serializer,
            {
                unreachable!("satisfy Item::load requirements")
            }
        }

        STORAGE_V0
            .load(storage)
            .map_err(ContractError::LoadConfigV0)
            .and_then(|old_config| Config::new(old_config.price_config).store(storage))
    }
}
