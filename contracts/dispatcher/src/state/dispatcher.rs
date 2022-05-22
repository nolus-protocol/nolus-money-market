use cosmwasm_std::{StdResult, Storage, Timestamp};
use cw_storage_plus::Item;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ContractError;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct DispatchLog {
    pub last_dispatch: Timestamp,
}

impl DispatchLog {
    const STORAGE: Item<'static, Self> = Item::new("dispatch_log");

    pub fn new(last_dispatch: Timestamp) -> Self {
        DispatchLog { last_dispatch }
    }

    pub fn store(self, storage: &mut dyn Storage) -> StdResult<()> {
        Self::STORAGE.save(storage, &self)
    }

    pub fn load(storage: &dyn Storage) -> StdResult<Self> {
        Self::STORAGE.load(storage)
    }

    pub fn update(
        storage: &mut dyn Storage,
        last_dispatch: Timestamp,
    ) -> Result<(), ContractError> {
        Self::load(storage)?;
        Self::STORAGE.update(storage, |mut log| -> Result<DispatchLog, ContractError> {
            log.last_dispatch = last_dispatch;
            Ok(log)
        })?;
        Ok(())
    }
}
