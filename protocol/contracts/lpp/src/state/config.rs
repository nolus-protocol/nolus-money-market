use std::mem;

use serde::{Deserialize, Serialize};

use finance::percent::Percent100;
use platform::contract::Code;
use sdk::{cosmwasm_std::Storage, cw_storage_plus::Item};

use crate::{borrow::InterestRate, config::Config as ApiConfig, contract::Result};

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
pub struct Config {
    lease_code: Code,
    borrow_rate: InterestRate,
    min_utilization: BoundToHundredPercent,
    lease_code_admin: Addr,
}

impl Config {
    const STORAGE: Item<ApiConfig> = Item::new("config");

    pub fn new<Lpn>(msg: InstantiateMsg, lease_code: Code) -> Self
    where
        Lpn: CurrencyDef,
        Lpn::Group: MemberOf<Lpns>,
    {
        debug_assert_eq!(Ok(()), msg.lpn.of_currency(Lpn::dto()));
        Self {
            lease_code,
            borrow_rate: msg.borrow_rate,
            min_utilization: msg.min_utilization,
            lease_code_admin: msg.lease_code_admin,
        }
    }

    #[cfg(test)]
    pub fn new_unchecked(
        lease_code: Code,
        borrow_rate: InterestRate,
        min_utilization: BoundToHundredPercent,
        lease_code_admin: Addr,
    ) -> Self {
        Self {
            lease_code,
            borrow_rate,
            min_utilization,
            lease_code_admin,
        }
    }

    pub const fn lease_code(&self) -> Code {
        self.lease_code
    }

    pub const fn lease_code_admin(&self) -> Addr {
        self.lease_code_admin
    }

    pub const fn borrow_rate(&self) -> &InterestRate {
        &self.borrow_rate
    }

    pub const fn min_utilization(&self) -> BoundToHundredPercent {
        self.min_utilization
    }

    pub fn store(&self, storage: &mut dyn Storage) -> Result<()> {
        Self::STORAGE.save(storage, self).map_err(Into::into)
    }

    pub fn load(storage: &dyn Storage) -> Result<Self> {
        Self::STORAGE.load(storage).map_err(Into::into)
    }

    pub fn update_lease_code(storage: &mut dyn Storage, lease_code: Code) -> Result<()> {
        Self::update_field(storage, |config| {
            ApiConfig::new(lease_code, *config.borrow_rate(), config.min_utilization())
        })
    }

    pub fn update_borrow_rate(storage: &mut dyn Storage, borrow_rate: InterestRate) -> Result<()> {
        Self::update_field(storage, |config| {
            ApiConfig::new(config.lease_code(), borrow_rate, config.min_utilization())
        })
    }

    pub fn update_min_utilization(
        storage: &mut dyn Storage,
        min_utilization: Percent100,
    ) -> Result<()> {
        Self::update_field(storage, |config| {
            ApiConfig::new(config.lease_code(), *config.borrow_rate(), min_utilization)
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
