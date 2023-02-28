use super::{MigrationSpec, GeneralContracts, LpnContracts};

pub type MigrateContract = MigrationSpec<String>;
pub type MaybeMigrateContract = Option<MigrateContract>;

pub type MigrateGeneralContract = MigrationSpec<String>;
pub type MaybeMigrateGeneralContract = Option<MigrateGeneralContract>;
pub type MigrateGeneralContracts = GeneralContracts<MaybeMigrateGeneralContract>;

pub type MigrateLpnContract = MigrationSpec<String>;
pub type MaybeMigrateLpnContract = Option<MigrateLpnContract>;
pub type MigrateLpnContracts = LpnContracts<MaybeMigrateLpnContract>;
