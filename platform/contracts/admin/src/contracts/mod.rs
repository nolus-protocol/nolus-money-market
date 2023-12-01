use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use ::platform::contract::CodeId;
use sdk::{
    cosmwasm_std::Addr,
    schemars::{self, JsonSchema},
};

pub use self::{
    platform::PlatformTemplate,
    protocol::{Protocol, ProtocolTemplate},
};

#[cfg(feature = "contract")]
pub(crate) use self::impl_mod::migrate;

#[cfg(feature = "contract")]
mod impl_mod;
mod platform;
mod protocol;

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ContractsTemplate<T, U = ProtocolTemplate<BTreeMap<String, T>>> {
    pub platform: PlatformTemplate<T>,
    pub protocol: U,
}

pub type ContractsMigration = ContractsTemplate<Option<MigrationSpec>>;

pub type ContractsPostMigrationExecute = ContractsTemplate<Option<String>>;

pub(crate) type ContractsGroupedByProtocol = ContractsTemplate<Addr, BTreeMap<String, Protocol>>;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct MigrationSpec {
    pub code_id: CodeId,
    pub migrate_msg: String,
}
