use crate::{ProtocolRelease, SoftwarePackageRelease, VersionSegment};

use super::ProtocolPackageRelease;

impl ProtocolPackageRelease {
    pub const fn current(name: &'static str, version: &str, storage: VersionSegment) -> Self {
        Self {
            software: SoftwarePackageRelease::current(name, version, storage),
            protocol: ProtocolRelease::current(),
        }
    }
}
