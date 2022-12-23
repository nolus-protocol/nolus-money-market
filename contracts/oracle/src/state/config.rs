use serde::{Deserialize, Serialize};

use finance::{currency::SymbolOwned, duration::Duration, percent::Percent};
use marketprice::config::Config as PriceConfig;
use sdk::{
    cosmwasm_std::{StdResult, Storage},
    cw_storage_plus::Item,
    schemars::{self, JsonSchema},
};

use crate::ContractError;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema)]
pub struct Config {
    pub base_asset: SymbolOwned,
    pub price_config: PriceConfig,
}

impl Config {
    const STORAGE: Item<'static, Self> = Item::new("config");

    pub fn new(
        base_asset: SymbolOwned,
        price_feed_period: Duration,
        expected_feeders: Percent,
    ) -> Result<Self, ContractError> {
        Self::validate(price_feed_period, expected_feeders)?;
        Ok(Config {
            base_asset,
            price_config: PriceConfig::new(price_feed_period, expected_feeders),
        })
    }

    fn validate(
        price_feed_period: Duration,
        expected_feeders: Percent,
    ) -> Result<(), ContractError> {
        if expected_feeders == Percent::ZERO || expected_feeders > Percent::HUNDRED {
            return Err(ContractError::Configuration(
                "Percent of expected available feeders should be > 0 and <= 1000".to_string(),
            ));
        }
        if price_feed_period == Duration::from_secs(0) {
            return Err(ContractError::Configuration(
                "Price feed period can not be 0".to_string(),
            ));
        }
        Ok(())
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
            c.price_config = PriceConfig::new(price_feed_period, expected_feeders);
            Ok(c)
        })?;
        Ok(())
    }
}
