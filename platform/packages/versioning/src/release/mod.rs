use serde::{Deserialize, Serialize};

use sdk::cosmwasm_std::Storage;

use crate::{Error, ProtocolRelease, SoftwarePackageRelease};

pub use self::id::Id;
use self::query::ProtocolPackage;

#[cfg(feature = "protocol_contract")]
mod current;
mod id;
pub mod query;

pub trait UpdatablePackage
where
    Self: Sized,
    Self::VersionQuery: Serialize + 'static,
{
    type VersionQuery;

    const VERSION_QUERY: &'static Self::VersionQuery;

    type ReleaseId;

    fn update_software(&self, to: &Self, to_release: &Self::ReleaseId) -> Result<(), Error>;

    fn update_software_and_storage<MigrateStorageFunctor, ContractError, MapErrorFunctor>(
        &self,
        to: &Self,
        to_release: &Self::ReleaseId,
        storage: &mut dyn Storage,
        migrate_storage: MigrateStorageFunctor,
        map_error: MapErrorFunctor,
    ) -> Result<(), ContractError>
    where
        MigrateStorageFunctor: FnOnce(&mut dyn Storage) -> Result<(), ContractError>,
        MapErrorFunctor: Fn(Error) -> ContractError;
}

pub type PlatformPackageRelease = SoftwarePackageRelease;
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct ProtocolPackageRelease {
    software: SoftwarePackageRelease,
    protocol: ProtocolRelease,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct ProtocolPackageReleaseId {
    software: Id,
    protocol: Id,
}

impl ProtocolPackageReleaseId {
    pub const VOID: Self = Self {
        software: Id::VOID,
        protocol: Id::VOID,
    };

    #[inline]
    pub const fn new(software: Id, protocol: Id) -> Self {
        Self { software, protocol }
    }
}

impl UpdatablePackage for ProtocolPackageRelease {
    type VersionQuery = ProtocolPackage;

    const VERSION_QUERY: &'static Self::VersionQuery = &ProtocolPackage::Release {};

    type ReleaseId = ProtocolPackageReleaseId;

    fn update_software(&self, to: &Self, to_release: &Self::ReleaseId) -> Result<(), Error> {
        self.protocol
            .check_update_allowed(&to.protocol, &to_release.protocol)
            .and_then(|_| {
                self.software
                    .update_software(&to.software, &to_release.software)
            })
    }

    fn update_software_and_storage<MigrateStorageFunctor, ContractError, MapErrorFunctor>(
        &self,
        to: &Self,
        to_release: &Self::ReleaseId,
        storage: &mut dyn Storage,
        migrate_storage: MigrateStorageFunctor,
        map_error: MapErrorFunctor,
    ) -> Result<(), ContractError>
    where
        MigrateStorageFunctor: FnOnce(&mut dyn Storage) -> Result<(), ContractError>,
        MapErrorFunctor: Fn(Error) -> ContractError,
    {
        self.protocol
            .check_update_allowed(&to.protocol, &to_release.protocol)
            .map_err(&map_error)
            .and_then(|()| {
                self.software.update_software_and_storage(
                    &to.software,
                    &to_release.software,
                    storage,
                    migrate_storage,
                    map_error,
                )
            })
    }
}
