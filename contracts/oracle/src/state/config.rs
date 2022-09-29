use cosmwasm_std::{Addr, StdResult, Storage};
use cw_storage_plus::Item;

use finance::{duration::Duration, percent::Percent};

use crate::ContractError;

use super::Config;

impl Config {
    const STORAGE: Item<'static, Self> = Item::new("config");

    pub fn new(
        denom: String,
        owner: Addr,
        price_feed_period: Duration,
        expected_feeders: Percent,
        timealarms_contract: Addr,
    ) -> Self {
        Config {
            base_asset: denom,
            owner,
            price_feed_period,
            expected_feeders,
            timealarms_contract,
        }
    }

    pub fn validate(self) -> Result<Config, ContractError> {
        if self.expected_feeders == Percent::ZERO || self.expected_feeders > Percent::HUNDRED {
            return Err(ContractError::Configuration(
                "Percent of expected available feeders should be > 0 and <= 1000".to_string(),
            ));
        }
        if self.price_feed_period == Duration::from_secs(0) {
            return Err(ContractError::Configuration(
                "Price feed period can not be 0".to_string(),
            ));
        }
        Ok(self)
    }

    pub fn store(self, storage: &mut dyn Storage) -> StdResult<()> {
        Self::STORAGE.save(storage, &self)
    }

    pub fn load(storage: &dyn Storage) -> StdResult<Self> {
        Self::STORAGE.load(storage)
    }

    pub fn update(
        storage: &mut dyn Storage,
        price_feed_period: Duration,
        expected_feeders: Percent,
    ) -> Result<(), ContractError> {
        Self::STORAGE.update(storage, |mut c| -> StdResult<_> {
            c.price_feed_period = price_feed_period;
            c.expected_feeders = expected_feeders;
            Ok(c)
        })?;
        Ok(())
    }

    pub fn set_time_alarms_address(
        storage: &mut dyn Storage,
        timealarms_contract: Addr,
        sender: Addr,
    ) -> Result<(), ContractError> {
        let config = Self::STORAGE.load(storage)?;
        if sender != config.owner {
            return Err(ContractError::Unauthorized {});
        }
        Self::STORAGE.update(storage, |mut c| -> StdResult<_> {
            c.timealarms_contract = timealarms_contract;
            Ok(c)
        })?;
        Ok(())
    }
}
