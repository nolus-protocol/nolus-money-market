use crate::{Error, ProtocolRelease, SoftwarePackageRelease, VersionSegment};

pub trait UpdatablePackage
where
    Self: Sized,
{
    fn update_software(&self, to: &Self) -> Result<(), Error>;

    fn update_software_and_storage(&self, to: &Self) -> Result<(), Error>;
}

pub type PlatformPackageRelease = SoftwarePackageRelease;
pub struct ProtocolPackageRelease {
    software: SoftwarePackageRelease,
    protocol: ProtocolRelease,
}

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

impl UpdatablePackage for ProtocolPackageRelease {
    fn update_software(&self, to: &Self) -> Result<(), Error> {
        self.protocol
            .check_update_allowed(&to.protocol)
            .and_then(|_| self.software.update_software(&to.software))
    }

    fn update_software_and_storage(&self, to: &Self) -> Result<(), Error> {
        self.protocol
            .check_update_allowed(&to.protocol)
            .and_then(|_| self.software.update_software_and_storage(&to.software))
    }
}
