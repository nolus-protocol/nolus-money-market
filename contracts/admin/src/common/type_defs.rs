use std::collections::HashMap;

use finance::currency::SymbolOwned;

use super::{CodeIdWithMigrateMsg, GeneralContracts, LpnContracts};

pub type MigrateContract = CodeIdWithMigrateMsg<String>;

pub type MigrateGeneralContract = CodeIdWithMigrateMsg<String>;
pub type MaybeMigrateGeneralContract = Option<MigrateGeneralContract>;
pub type MigrateGeneralContracts = GeneralContracts<MaybeMigrateGeneralContract>;

pub type MigrateLpnContract = CodeIdWithMigrateMsg<HashMap<SymbolOwned, String>>;
pub type MaybeMigrateLpnContract = Option<MigrateLpnContract>;
pub type MigrateLpnContracts = LpnContracts<MaybeMigrateLpnContract>;
