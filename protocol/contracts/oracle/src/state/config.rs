use std::mem;

use marketprice::config::Config as PriceConfig;
use sdk::{
    cosmwasm_std::{StdResult, Storage},
    cw_storage_plus::Item,
};

use crate::{api::Config, ContractError};

impl Config {
    const STORAGE: Item<'static, Self> = Item::new("config");

    pub fn new(price_config: PriceConfig) -> Self {
        Self { price_config }
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
        Self::STORAGE
            .update(storage, |mut c| -> StdResult<_> {
                c.price_config = price_config;
                Ok(c)
            })
            .map(mem::drop)
            .map_err(ContractError::UpdateConfig)
    }
}
