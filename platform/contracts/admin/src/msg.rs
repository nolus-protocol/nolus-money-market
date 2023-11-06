use serde::{Deserialize, Serialize};

use platform::contract::CodeId;
use sdk::{
    cosmwasm_std::Addr,
    schemars::{self, JsonSchema},
};

use crate::common::{
    type_defs::{
        ContractsMigration, ContractsPostMigrationExecute, MigrateContract,
        UncheckedContractsGroupedByDex,
    },
    Protocol,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields, untagged)]
pub enum InstantiateMsg {
    Instantiate {
        contract_owner: Addr,
        contracts: UncheckedContractsGroupedByDex,
    },
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields, untagged)]
pub enum MigrateMsg {
    Migrate { dex: String, contract_owner: Addr },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub enum ExecuteMsg {
    Instantiate {
        code_id: CodeId,
        label: String,
        message: String,
    },
    AddProtocolSet {
        dex: String,
        contracts: Protocol<Addr>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum SudoMsg {
    ChangeOwner {
        address: Addr,
    },
    AddProtocolSet {
        dex: String,
        contracts: Protocol<Addr>,
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
