use cosmwasm_std::{Addr, StdResult, Storage};
use cw_storage_plus::Item;
use marketprice::feed::{Denom, DenomPair};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ContractError;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub base_asset: Denom,
    pub owner: Addr,
    pub price_feed_period: u64,
    pub feeders_percentage_needed: u8,
    pub supported_denom_pairs: Vec<DenomPair>,
}

impl Config {
    const STORAGE: Item<'static, Self> = Item::new("config");

    pub fn new(
        denom: String,
        owner: Addr,
        price_feed_period: u64,
        feeders_percentage_needed: u8,
        supported_denom_pairs: Vec<DenomPair>,
    ) -> Self {
        Config {
            base_asset: denom,
            owner,
            price_feed_period,
            feeders_percentage_needed,
            supported_denom_pairs,
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
        price_feed_period: u64,
        feeders_percentage_needed: u8,
        sender: Addr,
    ) -> Result<(), ContractError> {
        let config = Self::STORAGE.load(storage)?;
        if sender != config.owner {
            return Err(ContractError::Unauthorized {});
        }
        Self::STORAGE.update(storage, |mut c| -> StdResult<_> {
            c.price_feed_period = price_feed_period;
            c.feeders_percentage_needed = feeders_percentage_needed;
            Ok(c)
        })?;
        Ok(())
    }
    pub fn update_supported_pairs(
        storage: &mut dyn Storage,
        pairs: Vec<DenomPair>,
        sender: Addr,
    ) -> Result<(), ContractError> {
        let config = Self::STORAGE.load(storage)?;
        if sender != config.owner {
            return Err(ContractError::Unauthorized {});
        }
        for pair in &pairs {
            if pair.0.eq_ignore_ascii_case(pair.1.as_str()) {
                return Err(ContractError::InvalidDenomPair(pair.to_owned()));
            }
        }

        Self::STORAGE.update(storage, |mut c| -> StdResult<_> {
            c.supported_denom_pairs = pairs;
            Ok(c)
        })?;
        Ok(())
    }
}
