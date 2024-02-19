use serde::{Deserialize, Serialize};

use platform::contract::CodeId;
use sdk::{
    cosmwasm_std::Addr,
    schemars::{self, JsonSchema},
};
use versioning::ReleaseLabel;

use crate::contracts::{
    ContractsGroupedByProtocol, ContractsMigration, ContractsPostMigrationExecute,
    PlatformTemplate, Protocol, ProtocolTemplate,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct InstantiateMsg {
    pub dex_admin: Addr,
    pub contracts: ContractsGroupedByProtocol,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct MigrateMsg {
    pub migrate_contracts: MigrateContracts,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub enum ExecuteMsg {
    Instantiate {
        code_id: CodeId,
        expected_address: Addr,
        protocol: String,
        label: String,
        message: String,
    },
    RegisterProtocol {
        name: String,
        protocol: Protocol,
    },
    /// A message for **internal purposes only**.
    ///
    /// It is meant to clean-up any temporary storage changes.
    ///
    /// Manual execution by an outside sender is considered an
    /// error, thus execution has to fail.
    EndOfMigration {},
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum SudoMsg {
    ChangeDexAdmin {
        new_dex_admin: Addr,
    },
    RegisterProtocol {
        name: String,
        protocol: Protocol,
    },
    /// Trigger a migration of contracts
    ///
    /// This message is meant to be used when the Admin contract
    /// itself does not need a migration. If one is needed then
    /// it should start as Admin contract migration which would then
    /// continue with the migration of the other contracts.
    MigrateContracts(MigrateContracts),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct MigrateContracts {
    pub release: ReleaseLabel,
    pub migration_spec: ContractsMigration,
    pub post_migration_execute: ContractsPostMigrationExecute,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum QueryMsg {
    InstantiateAddress { code_id: CodeId, protocol: String },
    Protocols {},
    Platform {},
    Protocol { protocol: String },
}

pub type ProtocolsQueryResponse = Vec<String>;

pub type PlatformQueryResponse = PlatformTemplate<Addr>;

pub type ProtocolQueryResponse = Protocol;

pub type ProtocolContracts = ProtocolTemplate<Addr>;
