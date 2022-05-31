use cosmwasm_std::{Coin, StdResult, Storage, Timestamp};
use cw_storage_plus::Item;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ContractError;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct TransferLog {
    pub last_transfer: Timestamp,
    pub amount: Vec<Coin>,
}

impl TransferLog {
    const STORAGE: Item<'static, Self> = Item::new("transfer_log");

    pub fn new(last_transfer: Timestamp) -> Self {
        TransferLog {
            last_transfer,
            amount: vec![],
        }
    }

    pub fn store(self, storage: &mut dyn Storage) -> StdResult<()> {
        Self::STORAGE.save(storage, &self)
    }
    pub fn load(storage: &dyn Storage) -> StdResult<Self> {
        Self::STORAGE.load(storage)
    }

    pub fn last_transfer(storage: &dyn Storage) -> StdResult<Timestamp> {
        match Self::STORAGE.load(storage) {
            Ok(l) => Ok(l.last_transfer),
            Err(_) => Ok(Timestamp::default()),
        }
    }

    pub fn update(
        storage: &mut dyn Storage,
        current_transfer: Timestamp,
        amount: &[Coin],
    ) -> Result<(), ContractError> {
        match Self::STORAGE.may_load(storage)? {
            None => Self::STORAGE.save(
                storage,
                &TransferLog {
                    last_transfer: current_transfer,
                    amount: amount.to_vec(),
                },
            )?,
            Some(l) => {
                if current_transfer < l.last_transfer {
                    return Err(ContractError::InvalidTimeConfiguration {});
                }
                Self::STORAGE.update(storage, |mut log| -> Result<TransferLog, ContractError> {
                    log.last_transfer = current_transfer;
                    Ok(log)
                })?;
            }
        }

        Ok(())
    }
}
