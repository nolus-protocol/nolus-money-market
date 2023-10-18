use serde::{de::DeserializeOwned, Deserialize, Serialize};

use currency::Currency;
use finance::{percent::bound::BoundToHundredPercent, price::Price};
use lpp_platform::NLpn;
use sdk::{
    cosmwasm_std::{Storage, Uint64},
    cw_storage_plus::Item,
};

use crate::{borrow::InterestRate, error::Result, msg::InstantiateMsg};

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
pub struct Config {
    lpn_ticker: String,
    lease_code_id: Uint64,
    borrow_rate: InterestRate,
    min_utilization: BoundToHundredPercent,
}

impl Config {
    const STORAGE: Item<'static, Self> = Item::new("config");

    #[cfg(any(feature = "migration", test))]
    pub const fn new(
        lpn_ticker: String,
        lease_code_id: Uint64,
        borrow_rate: InterestRate,
        min_utilization: BoundToHundredPercent,
    ) -> Self {
        Self {
            lpn_ticker,
            lease_code_id,
            borrow_rate,
            min_utilization,
        }
    }

    pub fn lpn_ticker(&self) -> &str {
        &self.lpn_ticker
    }

    pub const fn lease_code_id(&self) -> Uint64 {
        self.lease_code_id
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

    pub fn initial_derivative_price<Lpn>() -> Price<NLpn, Lpn>
    where
        Lpn: Currency + Serialize + DeserializeOwned,
    {
        Price::identity()
    }

    pub fn update_lease_code(storage: &mut dyn Storage, lease_code_id: Uint64) -> Result<()> {
        Self::update_field(storage, |config| Self {
            lease_code_id,
            ..config
        })
    }

    pub fn update_borrow_rate(storage: &mut dyn Storage, borrow_rate: InterestRate) -> Result<()> {
        Self::update_field(storage, |config| Self {
            borrow_rate,
            ..config
        })
    }

    pub fn update_min_utilization(
        storage: &mut dyn Storage,
        min_utilization: BoundToHundredPercent,
    ) -> Result<()> {
        Self::update_field(storage, |config| Self {
            min_utilization,
            ..config
        })
    }

    fn update_field<F>(storage: &mut dyn Storage, f: F) -> Result<()>
    where
        F: FnOnce(Config) -> Config,
    {
        Self::STORAGE
            .update(storage, |config: Self| Ok(f(config)))
            .map(|_| ())
    }
}

impl From<InstantiateMsg> for Config {
    fn from(msg: InstantiateMsg) -> Self {
        // 0 is a non-existing code id
        Self {
            lpn_ticker: msg.lpn_ticker,
            lease_code_id: Uint64::zero(),
            borrow_rate: msg.borrow_rate,
            min_utilization: msg.min_utilization,
        }
    }
}
