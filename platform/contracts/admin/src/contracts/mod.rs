use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use ::platform::contract::CodeId;
use sdk::{
    cosmwasm_std::{Addr, Uint64},
    schemars::{self, JsonSchema},
};

pub use self::{
    platform::PlatformTemplate,
    protocol::{Dex, Network, Protocol, ProtocolTemplate},
};

#[cfg(feature = "contract")]
pub(crate) use self::impl_mod::migrate;

#[cfg(feature = "contract")]
mod impl_mod;
mod platform;
mod protocol;

pub trait HigherOrderType {
    type Of<T>;
}

#[derive(Debug, Clone, Eq, PartialEq, JsonSchema)]
pub struct Identity;

impl HigherOrderType for Identity {
    type Of<T> = T;
}

#[derive(Debug, Clone, Eq, PartialEq, JsonSchema)]
pub struct HigherOrderOption;

impl HigherOrderType for HigherOrderOption {
    type Of<T> = Option<T>;
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(
    rename_all = "snake_case",
    deny_unknown_fields,
    bound(
        serialize = "OutmostHigherOrderType::Of<PlatformTemplate<PlatformUnit>>: Serialize, \
            OutmostHigherOrderType::Of<Protocol>: Serialize",
        deserialize = "OutmostHigherOrderType::Of<PlatformTemplate<PlatformUnit>>: Deserialize<'de>, \
            OutmostHigherOrderType::Of<Protocol>: Deserialize<'de>",
    )
)]
#[schemars(bound = "OutmostHigherOrderType: JsonSchema, \
    OutmostHigherOrderType::Of<PlatformTemplate<PlatformUnit>>: JsonSchema, \
    OutmostHigherOrderType::Of<Protocol>: JsonSchema, \
    PlatformUnit: JsonSchema, \
    Protocol: JsonSchema")]
pub struct ContractsTemplate<OutmostHigherOrderType, PlatformUnit, Protocol>
where
    OutmostHigherOrderType: HigherOrderType,
{
    pub platform: OutmostHigherOrderType::Of<PlatformTemplate<PlatformUnit>>,
    pub protocol: BTreeMap<String, OutmostHigherOrderType::Of<Protocol>>,
}

pub type ContractsMigration =
    ContractsTemplate<HigherOrderOption, MigrationSpec, ProtocolTemplate<MigrationSpec>>;

pub type Contracts = ContractsTemplate<Identity, Addr, Protocol>;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct MigrationSpec
where
    Uint64: Into<CodeId>,
    CodeId: Into<Uint64>,
{
    pub code_id: Uint64,
    pub migrate_msg: String,
    pub post_migrate_execute_msg: Option<String>,
}
