use sdk::{
    cosmwasm_std::{Addr, StdResult, Storage},
    cw_storage_plus::Item,
};

use crate::{error::ContractError, result::ContractResult};

use super::{reward_scale::RewardScale, CadenceHours, Config};

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
