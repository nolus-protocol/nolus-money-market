use std::mem;

use serde::{de::DeserializeOwned, Deserialize, Serialize};

use currency::Currency;
use finance::{percent::bound::BoundToHundredPercent, price::Price};
use lpp_platform::NLpn;
use platform::contract::Code;
use sdk::{cosmwasm_std::Storage, cw_storage_plus::Item};

use crate::{
    borrow::InterestRate,
    error::{ContractError, Result},
    msg::InstantiateMsg,
};

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
pub struct Config {
    lpn_ticker: String,
    #[serde(alias = "lease_code_id")]
    // TODO remove the alias once a new release with this change is deployed
    lease_code: Code,
    borrow_rate: InterestRate,
    min_utilization: BoundToHundredPercent,
}

impl Config {
    const STORAGE: Item<'static, Self> = Item::new("config");

    pub fn try_new<Lpn>(msg: InstantiateMsg, lease_code: Code) -> Result<Self>
    where
        Lpn: Currency,
    {
        if msg.lpn_ticker == Lpn::TICKER {
            Ok(Self {
                lpn_ticker: msg.lpn_ticker,
                lease_code,
                borrow_rate: msg.borrow_rate,
                min_utilization: msg.min_utilization,
            })
        } else {
            Err(ContractError::InvalidConfigParameter(
                "The LPN ticker does not match the LPN this contract is compiled with",
            ))
        }
    }

    #[cfg(test)]
    pub fn new_unchecked<Lpn>(
        lease_code: Code,
        borrow_rate: InterestRate,
        min_utilization: BoundToHundredPercent,
    ) -> Self
    where
        Lpn: Currency,
    {
        Self {
            lpn_ticker: Lpn::TICKER.into(),
            lease_code,
            borrow_rate,
            min_utilization,
        }
    }

    pub fn lpn_ticker(&self) -> &str {
        &self.lpn_ticker
    }

    pub const fn lease_code(&self) -> Code {
        self.lease_code
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

    pub fn update_lease_code(storage: &mut dyn Storage, lease_code: Code) -> Result<()> {
        Self::update_field(storage, |config| Self {
            lease_code,
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
            .map(mem::drop)
    }
}
