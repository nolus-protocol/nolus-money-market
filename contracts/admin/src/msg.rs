use serde::{Deserialize, Serialize};

use sdk::{
    cosmwasm_std::QuerierWrapper,
    schemars::{self, JsonSchema},
};

use crate::{
    common::type_defs::{
        Contracts, ContractsMigration, ContractsPostMigrationExecute, MigrateContract,
    },
    error::ContractError,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct InstantiateMsg {
    pub contracts: Contracts,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct MigrateMsg {}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum SudoMsg {
    MigrateContracts(MigrateContracts),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct MigrateContracts {
    pub release: String,
    pub admin_contract: Option<MigrateContract>,
    pub migration_spec: ContractsMigration,
    pub post_migration_execute: ContractsPostMigrationExecute,
}

impl InstantiateMsg {
    pub(crate) fn validate(&self, querier: &QuerierWrapper<'_>) -> Result<(), ContractError> {
        self.contracts.validate(querier)
    }
}
