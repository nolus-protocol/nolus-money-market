pub use crate::{
    error::Error,
    migration::MigrationMessage,
    protocol::Release as ProtocolRelease,
    release::{
        Id as ReleaseId, PlatformPackageRelease, ProtocolPackageRelease, ProtocolPackageReleaseId,
        UpdatablePackage, query,
    },
    software::{PackageRelease as SoftwarePackageRelease, VersionSegment},
};

mod error;
mod migration;
mod protocol;
mod release;
mod software;

pub type PlatformMigrationMessage<ContractMsg> =
    MigrationMessage<PlatformPackageRelease, ContractMsg>;

pub type ProtocolMigrationMessage<ContractMsg> =
    MigrationMessage<ProtocolPackageRelease, ContractMsg>;
