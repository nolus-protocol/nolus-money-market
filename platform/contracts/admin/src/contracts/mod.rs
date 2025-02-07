use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use ::platform::contract::CodeId;
use sdk::{
    cosmwasm_std::{Addr, Uint64},
    schemars::{self, JsonSchema},
};

#[cfg(feature = "contract")]
pub(crate) use self::impl_mod::{execute, migrate};
pub use self::{
    granular::{Granularity, HigherOrderType as HigherOrderGranularity},
    higher_order_type::{HigherOrderType, Option as HigherOrderOption},
    platform::{Contracts as PlatformContracts, HigherOrderType as HigherOrderPlatformContracts},
    protocol::{
        higher_order_type::{
            Contracts as HigherOrderProtocolContracts, Protocol as HigherOrderProtocol,
        },
        Contracts as ProtocolContracts, Dex, Network, Protocol,
    },
};

mod granular;
mod higher_order_type;
#[cfg(feature = "contract")]
mod impl_mod;
mod platform;
mod protocol;

#[cfg(feature = "contract")]
pub type PlatformContractAddresses = PlatformContracts<Addr>;

#[cfg(feature = "contract")]
pub type ProtocolContractAddresses = ProtocolContracts<Addr>;

pub type HigherOrderGranularOptional<T> = HigherOrderGranularity<T, HigherOrderOption>;

pub type HigherOrderGranularOptionalPlatformContracts =
    HigherOrderGranularOptional<HigherOrderPlatformContracts>;

pub type HigherOrderGranularOptionalProtocolContracts =
    HigherOrderGranularOptional<HigherOrderProtocolContracts>;

pub type HigherOrderPlatformMigration = HigherOrderGranularOptionalPlatformContracts;

#[cfg(feature = "contract")]
pub type PlatformMigration = <HigherOrderPlatformMigration as HigherOrderType>::Of<MigrationSpec>;

pub type HigherOrderProtocolMigration = HigherOrderGranularOptionalProtocolContracts;

#[cfg(feature = "contract")]
pub type ProtocolMigration = <HigherOrderProtocolMigration as HigherOrderType>::Of<MigrationSpec>;

pub type HigherOrderPlatformExecute = HigherOrderGranularOptionalPlatformContracts;

#[cfg(feature = "contract")]
pub type PlatformExecute = <HigherOrderPlatformExecute as HigherOrderType>::Of<String>;

pub type HigherOrderProtocolExecute = HigherOrderGranularOptionalProtocolContracts;

#[cfg(feature = "contract")]
pub type ProtocolExecute = <HigherOrderProtocolExecute as HigherOrderType>::Of<String>;

pub type Protocols<Protocol> = BTreeMap<String, Protocol>;

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(
    rename_all = "snake_case",
    deny_unknown_fields,
    bound(
        serialize = "Platform::Of<Unit>: Serialize, \
            Protocol::Of<Unit>: Serialize",
        deserialize = "Platform::Of<Unit>: Deserialize<'de>, \
            Protocol::Of<Unit>: Deserialize<'de>",
    )
)]
#[schemars(bound = "Platform: JsonSchema, \
    Platform::Of<Unit>: JsonSchema, \
    Protocol: JsonSchema, \
    Protocol::Of<Unit>: JsonSchema, \
    Unit: JsonSchema")]
pub struct ContractsTemplate<Platform, Protocol, Unit>
where
    Platform: HigherOrderType,
    Protocol: HigherOrderType,
{
    pub platform: Platform::Of<Unit>,
    pub protocol: Protocols<Protocol::Of<Unit>>,
}

pub type Contracts = ContractsTemplate<HigherOrderPlatformContracts, HigherOrderProtocol, Addr>;

pub type ContractsMigration =
    ContractsTemplate<HigherOrderPlatformMigration, HigherOrderProtocolMigration, MigrationSpec>;

pub type ContractsExecute =
    ContractsTemplate<HigherOrderPlatformExecute, HigherOrderProtocolExecute, String>;

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

#[cfg(test)]
const _: fn() = || {
    let _: ContractsExecute = ContractsExecute {
        platform: Granularity::All(Some(PlatformContracts {
            timealarms: String::new(),
            treasury: String::new(),
        })),
        protocol: BTreeMap::from([
            (
                String::new(),
                Granularity::Some {
                    some: ProtocolContracts {
                        leaser: Some(String::new()),
                        lpp: None,
                        oracle: Some(String::new()),
                        profit: Some(String::new()),
                        reserve: Some(String::new()),
                    },
                },
            ),
            (
                String::new(),
                Granularity::All(Some(ProtocolContracts {
                    leaser: String::new(),
                    lpp: String::new(),
                    oracle: String::new(),
                    profit: String::new(),
                    reserve: String::new(),
                })),
            ),
            (String::new(), Granularity::All(None)),
        ]),
    };
};
