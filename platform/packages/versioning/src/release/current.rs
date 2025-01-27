use crate::{Error, ProtocolRelease, SoftwarePackageRelease, VersionSegment};

use super::ProtocolPackageRelease;

impl ProtocolPackageRelease {
    pub const fn current(name: &'static str, version: &str, storage: VersionSegment) -> Self {
        Self {
            software: SoftwarePackageRelease::current(name, version, storage),
            protocol: ProtocolRelease::current(),
        }
    }

    pub fn pull_prev(
        package_name: &'static str,
        storage: &mut dyn sdk::cosmwasm_std::Storage,
    ) -> Result<Self, Error> {
        // since there is no info about the prev protocol we assume it is the current one
        SoftwarePackageRelease::pull_prev(package_name, storage).map(|software| Self {
            software,
            protocol: ProtocolRelease::current(),
        })
    }
}
