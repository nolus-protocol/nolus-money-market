use sdk::{
    cosmwasm_std::{Addr, StdResult, Storage},
    cw_storage_plus::Item,
};

use crate::{error::ContractError, result::ContractResult};

use super::{reward_scale::RewardScale, Config};

impl Config {
    const STORAGE: Item<'static, Self> = Item::new("dispatcher_config");

    pub fn new(
        cadence_hours: u16,
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

    pub fn update(storage: &mut dyn Storage, cadence_hours: u16) -> ContractResult<()> {
        Self::load(storage)?;

        Self::STORAGE
            .update(storage, |mut c| -> Result<Config, ContractError> {
                c.cadence_hours = cadence_hours;

                Ok(c)
            })
            .map(|_| ())
            .map_err(Into::into)
    }
}
