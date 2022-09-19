use cosmwasm_std::{Addr, StdResult, Storage};
use cw_storage_plus::Item;
use finance::currency::SymbolOwned;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ContractError;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct Config {
    pub base_asset: SymbolOwned,
    pub owner: Addr,
    pub price_feed_period_secs: u32,
    pub feeders_percentage_needed: u8,
    pub timealarms_contract: Addr,
}

impl Config {
    const STORAGE: Item<'static, Self> = Item::new("config");

    pub fn new(
        denom: String,
        owner: Addr,
        price_feed_period_secs: u32,
        feeders_percentage_needed: u8,
        timealarms_contract: Addr,
    ) -> Self {
        Config {
            base_asset: denom,
            owner,
            price_feed_period_secs,
            feeders_percentage_needed,
            timealarms_contract,
        }
    }

    pub fn store(self, storage: &mut dyn Storage) -> StdResult<()> {
        Self::STORAGE.save(storage, &self)
    }

    pub fn load(storage: &dyn Storage) -> StdResult<Self> {
        Self::STORAGE.load(storage)
    }

    pub fn update(
        storage: &mut dyn Storage,
        price_feed_period_secs: u32,
        feeders_percentage_needed: u8,
        sender: Addr,
    ) -> Result<(), ContractError> {
        let config = Self::STORAGE.load(storage)?;
        if sender != config.owner {
            return Err(ContractError::Unauthorized {});
        }
        Self::STORAGE.update(storage, |mut c| -> StdResult<_> {
            c.price_feed_period_secs = price_feed_period_secs;
            c.feeders_percentage_needed = feeders_percentage_needed;
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
