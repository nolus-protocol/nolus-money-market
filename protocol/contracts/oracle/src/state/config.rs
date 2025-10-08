use std::mem;

use currency::Group;
use marketprice::config::Config as PriceConfig;
use sdk::{
    cosmwasm_std::{StdResult, Storage},
    cw_storage_plus::Item,
};

use crate::{api::Config, error::Error, result::Result};

impl Config {
    const STORAGE: Item<Self> = Item::new("config");

    pub fn new(price_config: PriceConfig) -> Self {
        Self { price_config }
    }

    pub fn store<PriceG>(self, storage: &mut dyn Storage) -> Result<(), PriceG>
    where
        PriceG: Group,
    {
        Self::STORAGE
            .save(storage, &self)
            .map_err(Error::<PriceG>::store_config)
    }

    pub fn load<PriceG>(storage: &dyn Storage) -> Result<Self, PriceG>
    where
        PriceG: Group,
    {
        Self::STORAGE
            .load(storage)
            .map_err(Error::<PriceG>::load_config)
    }

    pub fn update<PriceG>(
        storage: &mut dyn Storage,
        price_config: PriceConfig,
    ) -> Result<(), PriceG>
    where
        PriceG: Group,
    {
        Self::STORAGE
            .update(storage, |mut c| -> StdResult<_> {
                c.price_config = price_config;
                Ok(c)
            })
            .map(mem::drop)
            .map_err(Error::<PriceG>::update_config)
    }
}
