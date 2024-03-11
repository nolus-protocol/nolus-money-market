use serde::{Deserialize, Serialize};

use platform::contract::CodeId;
use sdk::{
    cosmwasm_std::Storage,
    cw_storage_plus::Item,
    schemars::{self, JsonSchema},
};

use crate::{api::InstantiateMsg, error::ContractResult};

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

    pub fn store(&self, storage: &mut dyn Storage) -> ContractResult<()> {
        Self::STORAGE.save(storage, self).map_err(Into::into)
    }

    pub fn load(storage: &dyn Storage) -> ContractResult<Self> {
        Self::STORAGE.load(storage).map_err(Into::into)
    }

    pub fn update_lease_code(
        storage: &mut dyn Storage,
        lease_code_id: CodeId,
    ) -> ContractResult<()> {
        Self::update_field(storage, |_config| Self {
            lease_code_id,
            // ..config
        })
    }

    fn update_field<F>(storage: &mut dyn Storage, f: F) -> ContractResult<()>
    where
        F: FnOnce(Config) -> Config,
    {
        Self::STORAGE
            .update(storage, |config: Self| Ok(f(config)))
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
