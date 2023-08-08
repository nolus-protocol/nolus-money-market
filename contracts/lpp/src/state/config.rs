use serde::{de::DeserializeOwned, Deserialize, Serialize};

use currency::Currency;
use finance::{percent::Percent, price::Price};
use sdk::{
    cosmwasm_std::{Storage, Uint64},
    cw_storage_plus::Item,
};

use crate::{
    borrow::InterestRate,
    error::{ContractError, Result},
    msg::InstantiateMsg,
    nlpn::NLpn,
};

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
pub struct Config {
    lpn_ticker: String,
    lease_code_id: Uint64,
    borrow_rate: InterestRate,
    min_utilization: Percent,
}

impl Config {
    const STORAGE: Item<'static, Self> = Item::new("config");

    #[cfg(test)]
    pub const fn new(
        lpn_ticker: String,
        lease_code_id: Uint64,
        borrow_rate: InterestRate,
        min_utilization: Percent,
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

    pub const fn min_utilization(&self) -> Percent {
        self.min_utilization
    }

    pub fn store(&self, storage: &mut dyn Storage) -> Result<()> {
        Self::STORAGE.save(storage, self).map_err(Into::into)
    }

    pub fn load(storage: &dyn Storage) -> Result<Self> {
        Self::STORAGE.load(storage).map_err(Into::into)
    }

    pub fn update_lease_code(storage: &mut dyn Storage, lease_code: Uint64) -> Result<()> {
        Self::STORAGE
            .update::<_, ContractError>(storage, |mut config| {
                config.lease_code_id = lease_code;

                Ok(config)
            })
            .map(|_| ())
    }

    pub fn update_parameters(
        storage: &mut dyn Storage,
        borrow_rate: InterestRate,
        min_utilization: Percent,
    ) -> Result<()> {
        Self::STORAGE
            .update(storage, |config: Self| {
                Ok(Self {
                    borrow_rate,
                    min_utilization,
                    ..config
                })
            })
            .map(|_| ())
    }

    pub fn initial_derivative_price<LPN>() -> Price<NLpn, LPN>
    where
        LPN: Currency + Serialize + DeserializeOwned,
    {
        Price::identity()
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
