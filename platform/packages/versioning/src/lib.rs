use release::UpdatablePackage;
use sdk::cosmwasm_std::Storage;

#[cfg(feature = "schema")]
pub use crate::software::SemVer;
pub use crate::{
    error::Error,
    protocol::Release as ProtocolRelease,
    release::{PlatformPackageRelease, ProtocolPackageRelease},
    software::{PackageRelease as SoftwarePackageRelease, ReleaseId, VersionSegment},
};

mod error;
mod protocol;
mod release;
mod software;

pub fn update_software<PackageRelease>(
    previous: PackageRelease,
    current: PackageRelease,
) -> Result<ReleaseId, Error>
where
    PackageRelease: UpdatablePackage,
{
    previous
        .update_software(&current)
        //TODO remove the return value!!!
        .map(|()| ReleaseId::VOID)
}

pub struct FullUpdateOutput<MigrateStorageOutput> {
    pub to: ReleaseId,
    pub storage_migration_output: MigrateStorageOutput,
}

pub fn update_software_and_storage<
    PackageRelease,
    ContractError,
    MigrateStorageFunctor,
    StorageMigrationOutput,
    MapErrorFunctor,
>(
    storage: &mut dyn Storage,
    previous: PackageRelease,
    current: PackageRelease,
    migrate_storage: MigrateStorageFunctor,
    map_error: MapErrorFunctor,
) -> Result<FullUpdateOutput<StorageMigrationOutput>, ContractError>
where
    PackageRelease: UpdatablePackage,
    MigrateStorageFunctor:
        FnOnce(&mut dyn Storage) -> Result<StorageMigrationOutput, ContractError>,
    MapErrorFunctor: FnOnce(Error) -> ContractError,
{
    previous
        .update_software_and_storage(&current)
        .map_err(map_error)
        .and_then(|()| {
            migrate_storage(storage).map(|storage_migration_output| FullUpdateOutput {
                to: ReleaseId::VOID, //TODO remove the release return value!!!
                storage_migration_output,
            })
        })
}
