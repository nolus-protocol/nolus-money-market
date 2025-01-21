use sdk::cosmwasm_std::{StdError, Storage};

#[cfg(feature = "schema")]
pub use self::software::SemVer;
pub use self::software::{PackageRelease, ReleaseId, VersionSegment};

mod software;

pub fn update_legacy_software<ContractError, MapErrorFunctor>(
    storage: &mut dyn Storage,
    prev_name: &'static str,
    current: PackageRelease,
    map_error: MapErrorFunctor,
) -> Result<ReleaseId, ContractError>
where
    MapErrorFunctor: FnOnce(StdError) -> ContractError,
{
    PackageRelease::pull_prev(prev_name, storage)
        .and_then(|prev_release| prev_release.update_software(current))
        .map_err(map_error)
        .map(PackageRelease::release)
}

pub struct FullUpdateOutput<MigrateStorageOutput> {
    pub to: ReleaseId,
    pub storage_migration_output: MigrateStorageOutput,
}

pub fn update_legacy_software_and_storage<
    ContractError,
    MigrateStorageFunctor,
    StorageMigrationOutput,
    MapErrorFunctor,
>(
    storage: &mut dyn Storage,
    prev_name: &'static str,
    current: PackageRelease,
    migrate_storage: MigrateStorageFunctor,
    map_error: MapErrorFunctor,
) -> Result<FullUpdateOutput<StorageMigrationOutput>, ContractError>
where
    MigrateStorageFunctor:
        FnOnce(&mut dyn Storage) -> Result<StorageMigrationOutput, ContractError>,
    MapErrorFunctor: FnOnce(StdError) -> ContractError,
{
    PackageRelease::pull_prev(prev_name, storage)
        .and_then(|prev_release| prev_release.update_software_and_storage(current))
        .map_err(map_error)
        .and_then(|new_release| {
            migrate_storage(storage).map(|storage_migration_output| FullUpdateOutput {
                to: new_release.release(),
                storage_migration_output,
            })
        })
}
