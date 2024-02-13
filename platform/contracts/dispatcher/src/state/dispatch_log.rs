use serde::{Deserialize, Serialize};

use sdk::{
    cosmwasm_ext::as_dyn::{storage, AsDyn},
    cosmwasm_std::Timestamp,
    cw_storage_plus::Item,
    schemars::{self, JsonSchema},
};

use crate::ContractError;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct DispatchLog {
    pub last_dispatch: Timestamp,
}

impl DispatchLog {
    const STORAGE: Item<'static, Self> = Item::new("dispatch_log");

    pub fn new(last_dispatch: Timestamp) -> Self {
        DispatchLog { last_dispatch }
    }

    // TODO merge the functionality of this and `update`
    pub fn last_dispatch<S>(storage: &S) -> Timestamp
    where
        S: storage::Dyn + ?Sized,
    {
        Self::STORAGE
            .load(storage.as_dyn())
            .map(|log| log.last_dispatch)
            .unwrap_or_default()
    }

    pub fn update<S>(storage: &mut S, current_dispatch: Timestamp) -> Result<(), ContractError>
    where
        S: storage::DynMut + ?Sized,
    {
        match Self::STORAGE.may_load(storage.as_dyn())? {
            None => Self::STORAGE.save(
                storage.as_dyn_mut(),
                &DispatchLog {
                    last_dispatch: current_dispatch,
                },
            )?,
            Some(l) => {
                if current_dispatch < l.last_dispatch {
                    return Err(ContractError::InvalidTimeConfiguration {});
                }
                Self::STORAGE.update(
                    storage.as_dyn_mut(),
                    |mut log| -> Result<DispatchLog, ContractError> {
                        log.last_dispatch = current_dispatch;
                        Ok(log)
                    },
                )?;
            }
        }

        Ok(())
    }
}
