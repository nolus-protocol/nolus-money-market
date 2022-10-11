use sdk::{
    cosmwasm_std::{Addr, StdResult, Storage},
    cw_storage_plus::Item,
};

use crate::error::ContractError;

use super::{tvl_intervals::Intervals, Config};

impl Config {
    const STORAGE: Item<'static, Self> = Item::new("dispatcher_config");

    pub fn new(
        owner: Addr,
        cadence_hours: u16,
        lpp: Addr,
        oracle: Addr,
        timealarms: Addr,
        treasury: Addr,
        tvl_to_apr: Intervals,
    ) -> Self {
        Config {
            cadence_hours,
            owner,
            lpp,
            oracle,
            timealarms,
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

    pub fn update(storage: &mut dyn Storage, cadence_hours: u16) -> Result<(), ContractError> {
        Self::load(storage)?;
        Self::STORAGE.update(storage, |mut c| -> Result<Config, ContractError> {
            c.cadence_hours = cadence_hours;

            Ok(c)
        })?;
        Ok(())
    }
}
