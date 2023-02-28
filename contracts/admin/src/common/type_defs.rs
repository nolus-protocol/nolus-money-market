use super::MigrationSpec;

pub type MigrateContract = MigrationSpec<String>;
pub type MaybeMigrateContract = Option<MigrateContract>;
