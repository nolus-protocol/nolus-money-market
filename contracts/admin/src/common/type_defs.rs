use std::collections::HashMap;

use finance::currency::SymbolOwned;

use super::{CodeIdWithMigrateMsg, GeneralContractsGroup, SpecializedContractsGroup};

pub type MigrateInfo = CodeIdWithMigrateMsg<String>;

pub type MigrateGeneral = CodeIdWithMigrateMsg<String>;
pub type MaybeMigrateGeneral = Option<MigrateGeneral>;
pub type MigrateGeneralContracts = GeneralContractsGroup<MaybeMigrateGeneral>;

pub type MigrateSpecialized = CodeIdWithMigrateMsg<HashMap<SymbolOwned, String>>;
pub type MaybeMigrateSpecialized = Option<MigrateSpecialized>;
pub type MigrateSpecializedContracts = SpecializedContractsGroup<MaybeMigrateSpecialized>;
