use crate::{Error, ProtocolRelease, SoftwarePackageRelease};

pub use id::Id;

#[cfg(feature = "protocol_contract")]
mod current;
mod id;

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
