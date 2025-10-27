use serde::{Deserialize, Serialize};

use cosmwasm_std::Addr;
use sdk::{cosmwasm_std::Storage, cw_storage_plus::Item};

use crate::result::Result;

#[derive(Serialize, Deserialize, Clone, Eq, PartialEq)]
pub struct Config {
    contract_owner: Addr,
}

impl Config {
    const STORAGE: Item<Self> = Item::new("config");

    pub const fn new(contract_owner: Addr) -> Self {
        Self { contract_owner }
    }

    pub const fn contract_owner(&self) -> &Addr {
        &self.contract_owner
    }

    pub fn store(&self, storage: &mut dyn Storage) -> Result<()> {
        Self::STORAGE.save(storage, self).map_err(Into::into)
    }

    pub fn load(storage: &dyn Storage) -> Result<Self> {
        Self::STORAGE.load(storage).map_err(Into::into)
    }
}

#[cfg(test)]
mod test {
    use cosmwasm_std::{Addr, testing::MockStorage};

    use super::Config;

    #[test]
    fn store_load() {
        let admin = Addr::unchecked("admin");
        let mut store = MockStorage::new();
        assert_eq!(Ok(()), Config::new(admin.clone()).store(&mut store));
        assert_eq!(admin, Config::load(&store).unwrap().contract_owner());
    }
}
