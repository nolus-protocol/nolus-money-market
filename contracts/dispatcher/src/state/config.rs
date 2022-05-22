use std::cmp::Ordering;

use cosmwasm_std::{Addr, StdResult, Storage};
use cw_storage_plus::Item;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ContractError;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub cadence_hours: u32,
    pub owner: Addr,
    pub lpp: Addr,
    pub time_oracle: Addr,
    pub treasury: Addr,
    pub tvl_to_apr: Vec<TvlApr>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema, Eq)]
pub struct TvlApr {
    pub tvl: u32,
    pub apr: u32, //in permille
}

impl TvlApr {
    pub fn new(tvl: u32, apr: u32) -> Self {
        TvlApr { tvl, apr }
    }
}
impl Ord for TvlApr {
    fn cmp(&self, other: &Self) -> Ordering {
        self.tvl.cmp(&other.tvl)
    }
}

impl PartialOrd for TvlApr {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Config {
    const STORAGE: Item<'static, Self> = Item::new("dispatcher_config");

    pub fn new(
        owner: Addr,
        cadence_hours: u32,
        lpp: Addr,
        time_oracle: Addr,
        treasury: Addr,
        tvl_to_apr: Vec<TvlApr>,
    ) -> Self {
        Config {
            cadence_hours,
            owner,
            lpp,
            time_oracle,
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

    pub fn update(storage: &mut dyn Storage, cadence_hours: u32) -> Result<(), ContractError> {
        Self::load(storage)?;
        Self::STORAGE.update(storage, |mut c| -> Result<Config, ContractError> {
            c.cadence_hours = cadence_hours;

            Ok(c)
        })?;
        Ok(())
    }
}
