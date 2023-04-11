use serde::{Deserialize, Serialize};

use sdk::{
    cosmwasm_std::{Addr, Storage},
    cw_storage_plus::Item,
    schemars::{self, JsonSchema},
};

use crate::{result::ContractResult, ContractError};

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, JsonSchema)]
pub struct Config {
    pub cadence_hours: u16,
    pub treasury: Addr,
}

impl Config {
    const STORAGE: Item<'static, Self> = Item::new("profit_config");

    pub fn new(cadence_hours: u16, treasury: Addr) -> Self {
        Config {
            cadence_hours,
            treasury,
        }
    }

    pub fn store(self, storage: &mut dyn Storage) -> ContractResult<()> {
        Self::STORAGE.save(storage, &self).map_err(Into::into)
    }

    pub fn load(storage: &dyn Storage) -> ContractResult<Self> {
        Self::STORAGE.load(storage).map_err(Into::into)
    }

    pub fn update(storage: &mut dyn Storage, cadence_hours: u16) -> ContractResult<()> {
        Self::STORAGE.update(storage, |mut c| -> Result<Config, ContractError> {
            c.cadence_hours = cadence_hours;

            Ok(c)
        })?;

        Ok(())
    }
}
