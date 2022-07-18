use cosmwasm_std::{Addr, StdResult, Storage};
use cw_storage_plus::Item;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ContractError;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub cadence_hours: u16,
    pub owner: Addr,
    pub treasury: Addr,
    pub timealarms: Addr,
}

impl Config {
    const STORAGE: Item<'static, Self> = Item::new("profit_config");

    pub fn new(owner: Addr, cadence_hours: u16, treasury: Addr, timealarms: Addr) -> Self {
        Config {
            cadence_hours,
            owner,
            treasury,
            timealarms,
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
