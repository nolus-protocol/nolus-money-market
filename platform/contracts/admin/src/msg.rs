use serde::{Deserialize, Serialize};

use sdk::schemars::{self, JsonSchema};

use crate::common::type_defs::{
    ContractsMigration, ContractsPostMigrationExecute, MigrateContract, UncheckedContracts,
    UncheckedProtocolContracts,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields, untagged)]
pub enum InstantiateMsg {
    Instantiate { contracts: UncheckedContracts },
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields, untagged)]
pub enum MigrateMsg {
    Migrate { dex: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum SudoMsg {
    AddProtocolSet {
        dex: String,
        contracts: UncheckedProtocolContracts,
    },
    MigrateContracts(MigrateContracts),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields, untagged)]
pub enum MigrateContracts {
    MigrateContracts {
        release: String,
        admin_contract: Option<MigrateContract>,
        migration_spec: ContractsMigration,
        post_migration_execute: ContractsPostMigrationExecute,
    },
}
