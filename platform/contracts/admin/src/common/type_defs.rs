use sdk::cosmwasm_std::Addr as UncheckedAddr;

use super::{CheckedAddr, ContractsTemplate, MigrationSpec, Platform, Protocol};

pub type MigrateContract = MigrationSpec<String>;
pub type MaybeMigrateContract = Option<MigrateContract>;

pub(crate) type PlatformContracts = Platform<CheckedAddr>;
pub type UncheckedProtocolContracts = Protocol<UncheckedAddr>;
pub(crate) type ProtocolContracts = Protocol<CheckedAddr>;
pub type UncheckedContracts = ContractsTemplate<UncheckedAddr>;
pub(crate) type Contracts = ContractsTemplate<CheckedAddr>;

pub type PlatformContractsMigration = Platform<MaybeMigrateContract>;
pub type ProtocolContractsMigration = Protocol<MaybeMigrateContract>;
pub type ContractsMigration = ContractsTemplate<MaybeMigrateContract>;

pub type PlatformContractsPostMigrationExecute = Platform<Option<String>>;
pub type ProtocolContractsPostMigrationExecute = Protocol<Option<String>>;
pub type ContractsPostMigrationExecute = ContractsTemplate<Option<String>>;
