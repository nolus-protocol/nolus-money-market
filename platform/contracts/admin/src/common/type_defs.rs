use sdk::cosmwasm_std::Addr;

use super::{ContractsTemplate, Protocol, Platform, MigrationSpec};

pub type MigrateContract = MigrationSpec<String>;
pub type MaybeMigrateContract = Option<MigrateContract>;

pub type PlatformContracts = Platform<Addr>;
pub type ProtocolContracts = Protocol<Addr>;
pub type Contracts = ContractsTemplate<Addr>;

pub type PlatformContractsMigration = Platform<MaybeMigrateContract>;
pub type ProtocolContractsMigration = Protocol<MaybeMigrateContract>;
pub type ContractsMigration = ContractsTemplate<MaybeMigrateContract>;

pub type PlatformContractsPostMigrationExecute = Platform<Option<String>>;
pub type ProtocolContractsPostMigrationExecute = Protocol<Option<String>>;
pub type ContractsPostMigrationExecute = ContractsTemplate<Option<String>>;
