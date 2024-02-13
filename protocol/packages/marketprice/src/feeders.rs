use std::collections::HashSet;

use thiserror::Error;

use sdk::{
    cosmwasm_ext::as_dyn::storage,
    cosmwasm_std::{Addr, DepsMut, StdError, StdResult},
    cw_storage_plus::Item,
};

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
    pub const fn new(namespace: &'f str) -> Self {
        Self(Item::new(namespace))
    }

    pub fn get<S>(&self, storage: &S) -> StdResult<HashSet<Addr>>
    where
        S: storage::Dyn + ?Sized,
    {
        self.0
            .may_load(storage.as_dyn())
            .map(Option::unwrap_or_default)
    }

    pub fn is_registered<S>(&self, storage: &S, address: &Addr) -> StdResult<bool>
    where
        S: storage::Dyn + ?Sized,
    {
        self.0
            .may_load(storage.as_dyn())
            .map(|maybe_addrs: Option<HashSet<Addr>>| {
                maybe_addrs.map_or(false, |addrs: HashSet<Addr>| addrs.contains(address))
            })
    }

    pub fn register(&self, deps: DepsMut<'_>, address: Addr) -> Result<(), PriceFeedersError> {
        let mut db = self.0.may_load(deps.storage)?.unwrap_or_default();

        if db.contains(&address) {
            return Err(PriceFeedersError::FeederAlreadyRegistered {});
        }

        db.insert(address);

        self.0.save(deps.storage, &db)?;

        Ok(())
    }

    pub fn remove(&self, deps: DepsMut<'_>, feeder: &Addr) -> Result<(), PriceFeedersError> {
        let remove_address = |mut feeders: HashSet<Addr>| -> HashSet<Addr> {
            feeders.remove(feeder);
            feeders
        };

        if let Some(feeders) = self.0.may_load(deps.storage).transpose() {
            feeders
                .map(remove_address)
                .and_then(|new_feeders| self.0.save(deps.storage, &new_feeders))
                .map_err(Into::into)
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use sdk::cosmwasm_std::{testing, Addr};

    use crate::feeders::PriceFeeders;

    #[test]
    fn remove_empty() {
        let mut deps = testing::mock_dependencies();
        let feeders = PriceFeeders::new("storage_namespace");
        feeders
            .remove(deps.as_mut(), &Addr::unchecked("test_feeder"))
            .unwrap();
    }

    #[test]
    fn remove_existing() {
        let mut deps = testing::mock_dependencies();
        let feeders = PriceFeeders::new("storage_namespace");
        let new_feeder = Addr::unchecked("feeder34");
        feeders.register(deps.as_mut(), new_feeder.clone()).unwrap();
        assert_eq!(Ok(true), feeders.is_registered(&deps.storage, &new_feeder));

        feeders.remove(deps.as_mut(), &new_feeder).unwrap();

        assert_eq!(Ok(false), feeders.is_registered(&deps.storage, &new_feeder));
    }
}
