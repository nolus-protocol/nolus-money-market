use serde::{Deserialize, Serialize};

use platform::contract::CodeId;
use sdk::{
    cosmwasm_std::Storage,
    cw_storage_plus::Item,
    schemars::{self, JsonSchema},
};

use crate::{api::InstantiateMsg, error::Result};

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, JsonSchema)]
pub struct Config {
    lease_code_id: CodeId,
}

impl Config {
    const STORAGE: Item<'static, Self> = Item::new("config");

    #[cfg(test)]
    pub const fn new(lease_code_id: CodeId) -> Self {
        Self { lease_code_id }
    }

    pub const fn lease_code_id(&self) -> CodeId {
        self.lease_code_id
    }

    pub fn store(&self, storage: &mut dyn Storage) -> Result<()> {
        Self::STORAGE.save(storage, self).map_err(Into::into)
    }

    pub fn load(storage: &dyn Storage) -> Result<Self> {
        Self::STORAGE.load(storage).map_err(Into::into)
    }

    pub fn update_lease_code(storage: &mut dyn Storage, lease_code_id: CodeId) -> Result<()> {
        Self::STORAGE
            .update(storage, |_config: Self| Ok(Self { lease_code_id }))
            .map(|_| ())
    }
}

impl From<InstantiateMsg> for Config {
    fn from(msg: InstantiateMsg) -> Self {
        Self {
            lease_code_id: msg.lease_code_id.into(),
        }
    }
}

#[cfg(test)]
mod test {
    use cosmwasm_std::{testing::MockStorage, Storage};
    use platform::contract::CodeId;

    use crate::Config;

    #[test]
    fn store_load() {
        let lease_code_id = 12;
        let mut store = MockStorage::new();
        assert_eq!(Ok(()), Config::new(lease_code_id).store(&mut store));
        assert_lease_code_id(lease_code_id, &store);
    }

    #[test]
    fn update_load() {
        let lease_code_id = 28;
        let new_lease_code_id = lease_code_id + 10;
        let mut store = MockStorage::new();
        assert_eq!(Ok(()), Config::new(lease_code_id).store(&mut store));
        assert_eq!(
            Ok(()),
            Config::update_lease_code(&mut store, new_lease_code_id)
        );
        assert_lease_code_id(new_lease_code_id, &store);
    }

    fn assert_lease_code_id(lease_code_id: CodeId, store: &dyn Storage) {
        assert_eq!(lease_code_id, Config::load(store).unwrap().lease_code_id())
    }
}
