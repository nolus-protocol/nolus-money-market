use std::fmt::{self, Display, Formatter};

use serde::{Deserialize, Serialize};

use sdk::schemars::{self, JsonSchema};

use crate::{Error, ProtocolRelease, SoftwarePackageRelease};

pub(crate) use self::id::Id;

#[cfg(feature = "protocol_contract")]
mod current;
mod id;
pub mod query;

pub trait UpdatablePackage
where
    Self: Sized,
{
    type ReleaseId;

    fn update_software(&self, to: &Self, to_release: &Self::ReleaseId) -> Result<(), Error>;

    fn update_software_and_storage(
        &self,
        to: &Self,
        to_release: &Self::ReleaseId,
    ) -> Result<(), Error>;
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[repr(transparent)]
#[serde(transparent)]
pub struct SoftwareReleaseId(pub(crate) Id);

impl SoftwareReleaseId {
    pub const VOID: Self = Self(Id::VOID);

    #[cfg(feature = "testing")]
    #[inline]
    pub const fn new_test(s: &'static str) -> Self {
        Self(Id::new_static(s))
    }
}

impl Display for SoftwareReleaseId {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.0, f)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[repr(transparent)]
#[serde(transparent)]
pub struct ProtocolReleaseId(pub(crate) Id);

impl ProtocolReleaseId {
    pub const VOID: Self = Self(Id::VOID);

    #[cfg(feature = "testing")]
    #[inline]
    pub const fn new_test(s: &'static str) -> Self {
        Self(Id::new_static(s))
    }
}

impl Display for ProtocolReleaseId {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.0, f)
    }
}

pub type PlatformPackageRelease = SoftwarePackageRelease;
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct ProtocolPackageRelease {
    software: SoftwarePackageRelease,
    protocol: ProtocolRelease,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct ProtocolPackageReleaseId {
    software: SoftwareReleaseId,
    protocol: ProtocolReleaseId,
}

impl ProtocolPackageReleaseId {
    pub const VOID: Self = Self {
        software: SoftwareReleaseId::VOID,
        protocol: ProtocolReleaseId::VOID,
    };

    #[inline]
    pub const fn new(software: SoftwareReleaseId, protocol: ProtocolReleaseId) -> Self {
        Self { software, protocol }
    }
}

impl UpdatablePackage for ProtocolPackageRelease {
    type ReleaseId = ProtocolPackageReleaseId;

    fn update_software(&self, to: &Self, to_release: &Self::ReleaseId) -> Result<(), Error> {
        self.protocol
            .check_update_allowed(&to.protocol, &to_release.protocol)
            .and_then(|_| {
                self.software
                    .update_software(&to.software, &to_release.software)
            })
    }

    fn update_software_and_storage(
        &self,
        to: &Self,
        to_release: &Self::ReleaseId,
    ) -> Result<(), Error> {
        self.protocol
            .check_update_allowed(&to.protocol, &to_release.protocol)
            .and_then(|_| {
                self.software
                    .update_software_and_storage(&to.software, &to_release.software)
            })
    }
}
