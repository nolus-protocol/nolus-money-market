use std::mem;

use serde::{Deserialize, Serialize};

use platform::contract::Code;
use sdk::{cosmwasm_std::Storage, cw_storage_plus::Item};

use crate::error::Result;

#[derive(Serialize, Deserialize, Clone, Eq, PartialEq)]
pub struct Config {
    connection_id: String,
    dex_label: String,
    lease_code: Code,
}

impl Config {
    const STORAGE: Item<Self> = Item::new("config");

    pub fn new(connection_id: String, dex_label: String, lease_code: Code) -> Self {
        Self {
            connection_id,
            dex_label,
            lease_code,
        }
    }

    pub fn connection_id(&self) -> &str {
        &self.connection_id
    }

    pub fn dex_label(&self) -> &str {
        &self.dex_label
    }

    pub const fn lease_code(&self) -> Code {
        self.lease_code
    }

    pub(super) fn into_parts(self) -> (String, String, Code) {
        (self.connection_id, self.dex_label, self.lease_code)
    }

    pub fn store(&self, storage: &mut dyn Storage) -> Result<()> {
        Self::STORAGE.save(storage, self).map_err(Into::into)
    }

    pub fn load(storage: &dyn Storage) -> Result<Self> {
        Self::STORAGE.load(storage).map_err(Into::into)
    }

    pub fn update_lease_code(storage: &mut dyn Storage, lease_code: Code) -> Result<()> {
        Self::STORAGE
            .update(storage, |config: Self| {
                Ok(Self {
                    connection_id: config.connection_id,
                    dex_label: config.dex_label,
                    lease_code,
                })
            })
            .map(mem::drop)
    }
}

#[cfg(test)]
mod test {
    use platform::contract::{Code, CodeId};
    use sdk::cosmwasm_std::{Storage, testing::MockStorage};

    use super::Config;

    const CONNECTION_ID: &str = "connection-0";
    const DEX_LABEL: &str = "osmosis";

    #[test]
    fn store_load() {
        let lease_code = Code::unchecked(12);
        let mut store = MockStorage::new();
        config(lease_code).store(&mut store).unwrap();
        let loaded = Config::load(&store).unwrap();
        assert_eq!(CONNECTION_ID, loaded.connection_id());
        assert_eq!(DEX_LABEL, loaded.dex_label());
        assert_lease_code(lease_code, &store);
    }

    #[test]
    fn update_load() {
        let lease_code = Code::unchecked(28);
        let new_lease_code = Code::unchecked(CodeId::from(lease_code) + 10);
        let mut store = MockStorage::new();
        config(lease_code).store(&mut store).unwrap();
        Config::update_lease_code(&mut store, new_lease_code).unwrap();
        assert_lease_code(new_lease_code, &store);
        let loaded = Config::load(&store).unwrap();
        assert_eq!(CONNECTION_ID, loaded.connection_id());
        assert_eq!(DEX_LABEL, loaded.dex_label());
    }

    fn config(lease_code: Code) -> Config {
        Config::new(CONNECTION_ID.into(), DEX_LABEL.into(), lease_code)
    }

    fn assert_lease_code(expected: Code, store: &dyn Storage) {
        assert_eq!(expected, Config::load(store).unwrap().lease_code());
    }
}
