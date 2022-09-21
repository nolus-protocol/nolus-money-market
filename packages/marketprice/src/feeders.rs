use std::collections::HashSet;

use cosmwasm_std::{Addr, DepsMut, StdError, StdResult, Storage};
use cw_storage_plus::Item;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Errors returned from Feeders
#[derive(Error, Debug, PartialEq)]
pub enum PriceFeedersError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Given address already registered as a price feeder")]
    FeederAlreadyRegistered {},

    #[error("Given address not registered as a price feeder")]
    FeederNotRegistered {},

    #[error("Unauthorized")]
    Unauthorized {},
}

// state/logic
pub struct PriceFeeders<'f>(Item<'f, HashSet<Addr>>);

// this is the core business logic we expose
impl<'f> PriceFeeders<'f> {
    pub const fn new(namespace: &'f str) -> PriceFeeders {
        PriceFeeders(Item::new(namespace))
    }

    pub fn get(&self, storage: &dyn Storage) -> StdResult<HashSet<Addr>> {
        if self.0.may_load(storage)?.is_none() {
            return Err(StdError::generic_err("No registered feeders"));
        }
        let addrs = self.0.load(storage)?;
        Ok(addrs)
    }

    pub fn is_registered(&self, storage: &dyn Storage, address: &Addr) -> StdResult<bool> {
        if self.0.may_load(storage)?.is_none() {
            return Ok(false);
        }
        let addrs = self.0.load(storage)?;
        Ok(addrs.contains(address))
    }

    pub fn register(&self, deps: DepsMut, address: Addr) -> Result<(), PriceFeedersError> {
        let add_new_address = |mut addrs: HashSet<Addr>| -> StdResult<HashSet<Addr>> {
            addrs.insert(address.clone());
            Ok(addrs)
        };

        match self.0.may_load(deps.storage)? {
            None => self.0.save(deps.storage, &HashSet::from([address]))?,
            Some(_) => {
                self.0.update(deps.storage, add_new_address)?;
            }
        }

        Ok(())
    }

    pub fn remove(&self, deps: DepsMut, addr: Addr) -> Result<(), PriceFeedersError> {
        let remove_address = |mut addrs: HashSet<Addr>| -> StdResult<HashSet<Addr>> {
            addrs.remove(&addr);
            Ok(addrs)
        };

        if self.0.may_load(deps.storage)?.is_some() {
            self.0.update(deps.storage, remove_address)?;
        }

        Ok(())
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct FeedersResponse {
    pub addresses: HashSet<Addr>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct FeederResponse {
    pub exists: bool,
}
