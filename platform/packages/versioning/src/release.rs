use crate::{Error, ProtocolRelease, SoftwarePackageRelease};

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
    pub const fn new(software: SoftwarePackageRelease, protocol: ProtocolRelease) -> Self {
        Self { software, protocol }
    }

    pub fn pull_prev(
        package_name: &'static str,
        storage: &mut dyn sdk::cosmwasm_std::Storage,
        previous_protocol: ProtocolRelease,
    ) -> Result<Self, Error> {
        SoftwarePackageRelease::pull_prev(package_name, storage)
            .map(|package| Self::new(package, previous_protocol))
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
