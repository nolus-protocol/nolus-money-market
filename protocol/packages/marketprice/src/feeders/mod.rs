use std::{collections::HashSet, num::TryFromIntError};

use thiserror::Error;

use sdk::{
    cosmwasm_std::{Addr, Storage},
    cw_storage_plus::Item,
};

mod count;
pub use count::Count;

/// Errors returned from Feeders
#[derive(Error, Debug, PartialEq)]
pub enum PriceFeedersError {
    #[error("Failed to load price feeders: {cause}")]
    LoadingFailure { cause: String },

    #[error("Failed to save a price feeder: {cause}")]
    SavingFailure { cause: String },

    #[error("Given address already registered as a price feeder")]
    FeederAlreadyRegistered {},

    #[error("Given address not registered as a price feeder")]
    FeederNotRegistered {},

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Maximum feeder count exceeded: {0}")]
    FeederCountExceeded(TryFromIntError),

    #[error("Maximum feeder count reached")]
    MaxFeederCount {},
}

// state/logic
pub struct PriceFeeders(Item<HashSet<Addr>>);

// this is the core business logic we expose
impl PriceFeeders {
    pub const fn new(namespace: &'static str) -> Self {
        Self(Item::new(namespace))
    }

    pub fn feeders(&self, storage: &dyn Storage) -> Result<HashSet<Addr>, PriceFeedersError> {
        self.load(storage).map(Option::unwrap_or_default)
    }

    pub fn is_registered(
        &self,
        storage: &dyn Storage,
        address: &Addr,
    ) -> Result<bool, PriceFeedersError> {
        self.load(storage)
            .map(|maybe_addrs: Option<HashSet<Addr>>| {
                maybe_addrs.is_some_and(|addrs: HashSet<Addr>| addrs.contains(address))
            })
    }

    pub fn register(
        &self,
        storage: &mut dyn Storage,
        feeder: Addr,
    ) -> Result<(), PriceFeedersError> {
        let mut db = self.feeders(storage)?;

        count_of(&db).check_increment()?;

        (!db.contains(&feeder))
            .then_some(())
            .ok_or(PriceFeedersError::FeederAlreadyRegistered {})?;

        db.insert(feeder);

        self.0
            .save(storage, &db)
            .map_err(|ref err| PriceFeedersError::SavingFailure {
                cause: err.to_string(),
            })
    }

    pub fn remove(
        &self,
        storage: &mut dyn Storage,
        feeder: &Addr,
    ) -> Result<(), PriceFeedersError> {
        self.load(storage).and_then(|feeders| {
            feeders.map_or(const { Ok(()) }, |mut feeders| {
                feeders.remove(feeder);

                self.save(storage, &feeders)
            })
        })
    }

    pub fn total_registered(&self, storage: &dyn Storage) -> Result<Count, PriceFeedersError> {
        self.feeders(storage).map(|feeders| count_of(&feeders))
    }

    fn load(&self, storage: &dyn Storage) -> Result<Option<HashSet<Addr>>, PriceFeedersError> {
        self.0
            .may_load(storage)
            .map_err(|err| PriceFeedersError::LoadingFailure {
                cause: err.to_string(),
            })
    }

    fn save(
        &self,
        storage: &mut dyn Storage,
        feeders: &HashSet<Addr>,
    ) -> Result<(), PriceFeedersError> {
        self.0
            .save(storage, feeders)
            .map_err(|err| PriceFeedersError::SavingFailure {
                cause: err.to_string(),
            })
    }
}

fn count_of<T>(db: &HashSet<T>) -> Count {
    Count::try_from(db.len()).expect("registered feeders fit the allowed maximum!")
}

#[cfg(test)]
mod tests {
    use sdk::cosmwasm_std::{Addr, testing::MockStorage};

    use crate::feeders::PriceFeeders;

    #[test]
    fn remove_empty() {
        let mut storage = MockStorage::default();
        let feeders = PriceFeeders::new("storage_namespace");
        feeders
            .remove(&mut storage, &Addr::unchecked("test_feeder"))
            .unwrap();
    }

    #[test]
    fn remove_existing() {
        let mut storage = MockStorage::default();
        let feeders = PriceFeeders::new("storage_namespace");
        let new_feeder = Addr::unchecked("feeder34");
        feeders.register(&mut storage, new_feeder.clone()).unwrap();
        assert!(feeders.is_registered(&storage, &new_feeder).unwrap());

        feeders.remove(&mut storage, &new_feeder).unwrap();

        assert!(!feeders.is_registered(&storage, &new_feeder).unwrap());
    }
}
