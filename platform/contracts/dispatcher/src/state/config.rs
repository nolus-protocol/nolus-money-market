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
    // An LPP instance address
    pub lpp: Addr,
    // address to treasury contract
    pub treasury: Addr,
    // address to oracle contract
    pub oracle: Addr,
    // A list of (minTVL_MNLS: u32, APR%o) which defines the APR as per the TVL.
    pub tvl_to_apr: RewardScale,
}

impl Config {
    const STORAGE: Item<'static, Self> = Item::new("dispatcher_config");

    pub fn new(
        cadence_hours: CadenceHours,
        lpp: Addr,
        oracle: Addr,
        treasury: Addr,
        tvl_to_apr: RewardScale,
    ) -> Self {
        Config {
            cadence_hours,
            lpp,
            oracle,
            tvl_to_apr,
            treasury,
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
            .update(storage, |config| -> Result<Config, ContractError> {
                Ok(Self {
                    cadence_hours,
                    ..config
                })
            })
            .map(|_| ())
            .map_err(Into::into)
    }

    pub fn update_tvl_to_apr(
        storage: &mut dyn Storage,
        tvl_to_apr: RewardScale,
    ) -> ContractResult<()> {
        Self::STORAGE
            .update(storage, |config| -> Result<Config, ContractError> {
                Ok(Self {
                    tvl_to_apr,
                    ..config
                })
            })
            .map(|_| ())
            .map_err(Into::into)
    }
}
