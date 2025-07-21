use std::mem;

use serde::{Deserialize, Serialize};

use finance::percent::Percent100;
use platform::contract::Code;
use sdk::{cosmwasm_std::Storage, cw_storage_plus::Item};

use crate::{borrow::InterestRate, config::Config as ApiConfig, contract::Result};

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
#[serde(transparent)]
pub struct Config(ApiConfig);

impl Config {
    const STORAGE: Item<ApiConfig> = Item::new("config");

    pub fn store(cfg: &ApiConfig, storage: &mut dyn Storage) -> Result<()> {
        Self::STORAGE.save(storage, cfg).map_err(Into::into)
    }

    pub fn load(storage: &dyn Storage) -> Result<ApiConfig> {
        Self::STORAGE.load(storage).map_err(Into::into)
    }

    pub fn update_lease_code(storage: &mut dyn Storage, lease_code_new: Code) -> Result<()> {
        Self::update_field(storage, |config| {
            ApiConfig::new(
                lease_code_new,
                *config.borrow_rate(),
                config.min_utilization(),
                config.lease_code_admin().clone(),
            )
        })
    }

    pub fn update_borrow_rate(storage: &mut dyn Storage, borrow_rate: InterestRate) -> Result<()> {
        Self::update_field(storage, |config| {
            ApiConfig::new(
                config.lease_code(),
                borrow_rate,
                config.min_utilization(),
                config.lease_code_admin().clone(),
            )
        })
    }

    pub fn update_min_utilization(
        storage: &mut dyn Storage,
        min_utilization: Percent100,
    ) -> Result<()> {
        Self::update_field(storage, |config| {
            ApiConfig::new(
                config.lease_code(),
                *config.borrow_rate(),
                min_utilization,
                config.lease_code_admin().clone(),
            )
        })
    }

    fn update_field<F>(storage: &mut dyn Storage, f: F) -> Result<()>
    where
        F: FnOnce(ApiConfig) -> ApiConfig,
    {
        Self::STORAGE
            .update(storage, |config: ApiConfig| Ok(f(config)))
            .map(mem::drop)
    }
}
