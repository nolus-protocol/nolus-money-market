use serde::{Deserialize, Serialize};

use sdk::{
    cosmwasm_std::{Addr, StdResult, Storage},
    cw_storage_plus::Item,
};

use crate::{error::ContractError, result::ContractResult};

use super::reward_scale::RewardScale;

pub type CadenceHours = u16;

#[derive(Serialize, Deserialize)]
pub(crate) struct Config {
    // Time duration in hours defining the periods of time this instance is awaken
    pub cadence_hours: CadenceHours,
    // Protocols registry
    pub protocols_registry: Addr,
    // A list of (minTVL_MNLS: u32, APR%o) which defines the APR as per the TVL.
    pub tvl_to_apr: RewardScale,
}

impl Config {
    const STORAGE: Item<Self> = Item::new("dispatcher_config");

    pub fn new(
        cadence_hours: CadenceHours,
        protocols_registry: Addr,
        tvl_to_apr: RewardScale,
    ) -> Self {
        Config {
            cadence_hours,
            protocols_registry,
            tvl_to_apr,
        }
    }

    pub fn store(self, storage: &mut dyn Storage) -> StdResult<()> {
        Self::STORAGE.save(storage, &self)
    }

    pub fn load(storage: &dyn Storage) -> StdResult<Self> {
        Self::STORAGE.load(storage)
    }

    pub fn update_cadence_hours(
        storage: &mut dyn Storage,
        cadence_hours: CadenceHours,
    ) -> ContractResult<()> {
        Self::STORAGE
            .update(storage, |config| -> StdResult<Config> {
                Ok(Self {
                    cadence_hours,
                    ..config
                })
            })
            .map(|_| ())
            .map_err(ContractError::update_storage)
    }

    pub fn update_tvl_to_apr(
        storage: &mut dyn Storage,
        tvl_to_apr: RewardScale,
    ) -> ContractResult<()> {
        Self::STORAGE
            .update(storage, |config| -> StdResult<Config> {
                Ok(Self {
                    tvl_to_apr,
                    ..config
                })
            })
            .map(|_| ())
            .map_err(ContractError::update_storage)
    }
}
