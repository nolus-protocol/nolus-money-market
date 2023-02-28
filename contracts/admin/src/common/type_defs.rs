use super::{CodeIdWithMigrateMsg, GeneralContracts, LpnContracts};

pub type MigrateContract = CodeIdWithMigrateMsg<String>;
pub type MaybeMigrateContract = Option<MigrateContract>;

pub type MigrateGeneralContract = CodeIdWithMigrateMsg<String>;
pub type MaybeMigrateGeneralContract = Option<MigrateGeneralContract>;
pub type MigrateGeneralContracts = GeneralContracts<MaybeMigrateGeneralContract>;

pub type MigrateLpnContract = CodeIdWithMigrateMsg<String>;
pub type MaybeMigrateLpnContract = Option<MigrateLpnContract>;
pub type MigrateLpnContracts = LpnContracts<MaybeMigrateLpnContract>;
