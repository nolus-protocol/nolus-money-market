use std::collections::BTreeMap;

use sdk::cosmwasm_std::Addr as UncheckedAddr;

use super::{CheckedAddr, ContractsTemplate, MigrationSpec, Platform, Protocol};

pub type MigrateContract = MigrationSpec<String>;
pub type MaybeMigrateContract = Option<MigrateContract>;

pub(crate) type PlatformContracts = Platform<CheckedAddr>;
pub type UncheckedContractsGroupedByDex =
    ContractsTemplate<UncheckedAddr, BTreeMap<String, Protocol<UncheckedAddr>>>;
pub(crate) type ContractsGroupedByDex =
    ContractsTemplate<CheckedAddr, BTreeMap<String, Protocol<CheckedAddr>>>;

pub type PlatformContractsMigration = Platform<MaybeMigrateContract>;
pub type ContractsMigration = ContractsTemplate<MaybeMigrateContract>;

pub type PlatformContractsPostMigrationExecute = Platform<Option<String>>;
pub type ContractsPostMigrationExecute = ContractsTemplate<Option<String>>;
