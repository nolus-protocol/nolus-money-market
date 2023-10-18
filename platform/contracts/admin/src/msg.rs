use serde::{Deserialize, Serialize};

use sdk::{
    cosmwasm_std::{Addr, QuerierWrapper},
    schemars::{self, JsonSchema},
};

use crate::{
    common::{
        type_defs::{
            Contracts, ContractsMigration, ContractsPostMigrationExecute, MigrateContract,
        },
        Protocol,
    },
    error::Error,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct InstantiateMsg {
    pub contracts: Contracts,
}

impl InstantiateMsg {
    pub(crate) fn validate(&self, querier: &QuerierWrapper<'_>) -> Result<(), Error> {
        self.contracts.validate(querier)
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct MigrateMsg {
    pub dex: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum SudoMsg {
    AddProtocolSet {
        dex: String,
        contracts: Protocol<Addr>,
    },
    MigrateContracts(MigrateContracts),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct MigrateContracts {
    pub release: String,
    pub admin_contract: Option<MigrateContract>,
    pub migration_spec: ContractsMigration,
    pub post_migration_execute: ContractsPostMigrationExecute,
}
