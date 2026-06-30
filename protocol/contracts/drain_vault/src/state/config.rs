use serde::{Deserialize, Serialize};

use sdk::{cosmwasm_std::Addr, cw_storage_plus::Item};

use crate::error::Result;

#[derive(Serialize, Deserialize, Clone, Eq, PartialEq)]
pub struct Config {
    owner: Addr,
}

impl Config {
    const STORAGE: Item<Self> = Item::new("config");

    pub const fn new(owner: Addr) -> Self {
        Self { owner }
    }

    pub const fn owner(&self) -> &Addr {
        &self.owner
    }

    pub fn store(&self, storage: &mut dyn sdk::cosmwasm_std::Storage) -> Result<()> {
        Self::STORAGE.save(storage, self).map_err(Into::into)
    }

    pub fn load(storage: &dyn sdk::cosmwasm_std::Storage) -> Result<Self> {
        Self::STORAGE.load(storage).map_err(Into::into)
    }
}

#[cfg(test)]
mod test {
    use sdk::cosmwasm_std::{Addr, Storage, testing::MockStorage};

    use super::Config;

    #[test]
    fn store_load() {
        let owner = Addr::unchecked("profit");
        let mut store = MockStorage::new();
        Config::new(owner.clone()).store(&mut store).unwrap();
        assert_owner(&owner, &store);
    }

    fn assert_owner(owner: &Addr, store: &dyn Storage) {
        assert_eq!(owner, Config::load(store).unwrap().owner())
    }
}
