use sdk::cosmwasm_std::Addr;

use super::{ContractsTemplate, MigrationSpec};

pub type MigrateContract = MigrationSpec<String>;
pub type MaybeMigrateContract = Option<MigrateContract>;

pub type Contracts = ContractsTemplate<Addr>;
pub type ContractsMigration = ContractsTemplate<MaybeMigrateContract>;
