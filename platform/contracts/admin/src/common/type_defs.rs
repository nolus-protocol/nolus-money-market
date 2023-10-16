use sdk::cosmwasm_std::Addr;

use super::{ContractsTemplate, DexBound, DexIndependent, MigrationSpec};

pub type MigrateContract = MigrationSpec<String>;
pub type MaybeMigrateContract = Option<MigrateContract>;

pub type DexIndependentContracts = DexIndependent<Addr>;
pub type DexBoundContracts = DexBound<Addr>;
pub type Contracts = ContractsTemplate<Addr>;

pub type DexIndependentContractsMigration = DexIndependent<MaybeMigrateContract>;
pub type DexBoundContractsMigration = DexBound<MaybeMigrateContract>;
pub type ContractsMigration = ContractsTemplate<MaybeMigrateContract>;

pub type DexIndependentContractsPostMigrationExecute = DexIndependent<Option<String>>;
pub type DexBoundContractsPostMigrationExecute = DexBound<Option<String>>;
pub type ContractsPostMigrationExecute = ContractsTemplate<Option<String>>;
