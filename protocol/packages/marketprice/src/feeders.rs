use std::collections::HashSet;

use thiserror::Error;

use sdk::{
    cosmwasm_std::{Addr, DepsMut, StdError, StdResult, Storage},
    cw_storage_plus::Item,
};

/// Errors returned from Feeders
#[derive(Error, Debug, PartialEq)]
pub enum PriceFeedersError {
    #[error("{0}")]
    Std(String),

    #[error("Given address already registered as a price feeder")]
    FeederAlreadyRegistered {},

    #[error("Given address not registered as a price feeder")]
    FeederNotRegistered {},

    #[error("Unauthorized")]
    Unauthorized {},
}

impl From<StdError> for PriceFeedersError {
    fn from(value: StdError) -> Self {
        Self::Std(value.to_string())
    }
}

// state/logic
pub struct PriceFeeders(Item<HashSet<Addr>>);

// this is the core business logic we expose
impl PriceFeeders {
    pub const fn new(namespace: &'static str) -> Self {
        Self(Item::new(namespace))
    }

    pub fn get(&self, storage: &dyn Storage) -> StdResult<HashSet<Addr>> {
        self.0.may_load(storage).map(Option::unwrap_or_default)
    }

    pub fn is_registered(&self, storage: &dyn Storage, address: &Addr) -> StdResult<bool> {
        self.0
            .may_load(storage)
            .map(|maybe_addrs: Option<HashSet<Addr>>| {
                maybe_addrs.is_some_and(|addrs: HashSet<Addr>| addrs.contains(address))
            })
    }

    pub fn register(&self, deps: DepsMut<'_>, feeder: Addr) -> Result<(), PriceFeedersError> {
        let mut db = self.0.may_load(deps.storage)?.unwrap_or_default();

        if db.contains(&feeder) {
            return Err(PriceFeedersError::FeederAlreadyRegistered {});
        }

        db.insert(feeder);

        self.0.save(deps.storage, &db)?;

        Ok(())
    }

    pub fn remove(&self, deps: DepsMut<'_>, feeder: &Addr) -> Result<(), PriceFeedersError> {
        self.0
            .may_load(deps.storage)
            .and_then(|feeders| {
                feeders.map_or(const { Ok(()) }, |mut feeders| {
                    feeders.remove(feeder);

                    self.0.save(deps.storage, &feeders)
                })
            })
            .map_err(Into::into)
    }
}

#[cfg(test)]
mod tests {
    use sdk::cosmwasm_std::{Addr, StdError as CwError, testing};

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
        assert_eq!(
            Ok(&true),
            feeders
                .is_registered(&deps.storage, &new_feeder)
                .as_ref()
                .map_err(CwError::to_string)
        );

        feeders.remove(deps.as_mut(), &new_feeder).unwrap();

        assert_eq!(
            Ok(&false),
            feeders
                .is_registered(&deps.storage, &new_feeder)
                .as_ref()
                .map_err(CwError::to_string)
        );
    }
}
