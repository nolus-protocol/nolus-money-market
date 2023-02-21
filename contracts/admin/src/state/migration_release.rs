use sdk::{
    cosmwasm_std::{StdResult, Storage},
    cw_storage_plus::Item,
};

pub(crate) struct MigrationRelease;

impl MigrationRelease {
    const STORAGE_ITEM: Item<'_, String> = Item::new("migration_release");

    pub(crate) fn store(storage: &mut dyn Storage, migration_release: String) -> StdResult<()> {
        Self::STORAGE_ITEM.save(storage, &migration_release)
    }

    pub(crate) fn load(storage: &mut dyn Storage) -> StdResult<String> {
        Self::STORAGE_ITEM.load(storage)
    }
}
