use thiserror::Error;

use sdk::cosmwasm_std::StdError;

use crate::{protocol::Protocol, release::Id as ReleaseId, software::Package};

#[derive(Error, Debug, PartialEq)]
pub enum Error {
    #[error("[Versioning] {0}")]
    LoadPrevVersion(StdError),

    #[error(
        "[Versioning] The package release does not match the currently migration target! The current package's release is \"{0}\", the release being migrated to is \"{1}\"."
    )]
    SoftwareReleaseMismatch(ReleaseId, ReleaseId),

    #[error(
        "[Versioning] The protocol release does not match the currently migration target! The current protocol's release is \"{0}\", the release being migrated to is \"{1}\"."
    )]
    ProtocolReleaseMismatch(ReleaseId, ReleaseId),

    // TODO consider keeping &Package instead of String
    #[error(
        "[Versioning] The package names do not match! The current package is \"{0}\", the new package is \"{1}\"."
    )]
    PackageNamesMismatch(String, String),

    #[error(
        "[Versioning] The package does not increase monotonically! The current package is \"{0}\", the new package is \"{1}\"."
    )]
    OlderPackageCode(String, String),

    #[error(
        "[Versioning] The package storage versions do not match! The current package is \"{0}\", the new package is \"{1}\"."
    )]
    PackageStorageVersionMismatch(String, String),

    #[error(
        "[Versioning] The new package storage version is not adjacent to the current one! The current package is \"{0}\", the new package is \"{1}\"."
    )]
    PackageStorageVersionNotAdjacent(String, String),

    #[error(
        "[Versioning] The protocols do not match! The current package's protocol is \"{0}\", the new package's one is \"{1}\"."
    )]
    ProtocolMismatch(String, String),
}

impl Error {
    pub(crate) fn loading(cause: StdError) -> Self {
        Self::LoadPrevVersion(cause)
    }

    pub(crate) fn software_release_mismatch(current: ReleaseId, expected: ReleaseId) -> Self {
        Self::SoftwareReleaseMismatch(current, expected)
    }

    pub(crate) fn protocol_release_mismatch(current: ReleaseId, expected: ReleaseId) -> Self {
        Self::ProtocolReleaseMismatch(current, expected)
    }

    pub(crate) fn package_names_mismatch(current: &Package, new: &Package) -> Self {
        Self::PackageNamesMismatch(current.to_string(), new.to_string())
    }

    pub(crate) fn older_package_code(current: &Package, new: &Package) -> Self {
        Self::OlderPackageCode(current.to_string(), new.to_string())
    }

    pub(crate) fn package_storage_versions_mismatch(current: &Package, new: &Package) -> Self {
        Self::PackageStorageVersionMismatch(current.to_string(), new.to_string())
    }

    pub(crate) fn package_storage_version_not_adjacent(current: &Package, new: &Package) -> Self {
        Self::PackageStorageVersionNotAdjacent(current.to_string(), new.to_string())
    }

    pub(crate) fn protocol_mismatch(current: &Protocol, new: &Protocol) -> Self {
        Self::PackageNamesMismatch(current.to_string(), new.to_string())
    }
}
