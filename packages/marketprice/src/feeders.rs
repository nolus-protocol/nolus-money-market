use std::collections::HashSet;

use cosmwasm_std::{Addr, Deps, StdResult, StdError, DepsMut};
use cw_storage_plus::Item;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use thiserror::Error;


/// Errors returned from Admin
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

    pub fn get(&self, deps: Deps) -> StdResult<HashSet<Addr>> {
        let addrs = self.0.load(deps.storage)?;
        Ok( addrs )
    }

    pub fn is_registered(&self, deps: Deps, address: &Addr) -> StdResult<bool> {
        let addrs = self.0.load(deps.storage)?;
        Ok( addrs.contains(address) )
    }

    pub fn register(&self, deps: DepsMut, address: Addr) -> Result<(), PriceFeedersError> {

        let add_new_address = |mut addrs:HashSet<Addr>| -> StdResult<HashSet<Addr>> {
            addrs.insert(address.clone());
            Ok(addrs)
        };

        let can_load = self.0.may_load(deps.storage)?;
        match can_load {
            None => {
                let mut empty = HashSet::new();
                empty.insert(address);
                self.0.save(deps.storage, &empty)?
            },
            Some(_) => {
                self.0.update(deps.storage, add_new_address)?;
            }
        }


        Ok(())
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct FeedersResponse {
    pub addresses: HashSet<Addr>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct FeederResponse {
    pub exists: bool,
}




