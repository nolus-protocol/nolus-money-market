use serde::{Deserialize, Serialize};

use sdk::{
    cosmwasm_std::{StdError, Storage},
    cw_storage_plus::Item,
    schemars::{self, JsonSchema},
};

use crate::SemVer;

use super::Version;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema)]
#[repr(transparent)]
#[serde(transparent)]
pub struct ReleaseId(String);

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(test, derive(Debug))]
pub struct PackageRelease {
    id: ReleaseId,
    code: Version,
}

impl ReleaseId {
    const ID: &'static str = env!(
        "SOFTWARE_RELEASE_ID",
        "No software release identifier provided as an environment variable! Please set \
        \"SOFTWARE_RELEASE_ID\" environment variable!",
    );

    //TODO delete once deliver a version with contracts that take the current release as input
    const PREV: &'static str = "v0.7.6";

    const DEV: &'static str = "dev-release";

    const VOID: &'static str = "void-release";

    fn this() -> Self {
        Self(Self::ID.into())
    }

    fn prev() -> Self {
        Self(Self::PREV.into())
    }

    fn dev() -> Self {
        Self(Self::DEV.into())
    }

    fn void() -> Self {
        Self(Self::VOID.into())
    }
}

impl PackageRelease {
    pub fn void() -> Self {
        Self::instance(
            ReleaseId::void(),
            const { Version::new(0, SemVer::parse("0.0.0")) },
        )
    }

    pub(crate) fn this(code: Version) -> Self {
        Self::instance(ReleaseId::this(), code)
    }

    pub(crate) fn pull_prev(storage: &mut dyn Storage) -> Result<Self, StdError> {
        const VERSION_STORAGE_KEY: Item<Version> = Item::new("contract_version");

        VERSION_STORAGE_KEY
            .load(storage)
            .inspect(|_| VERSION_STORAGE_KEY.remove(storage))
            .map(|code| Self::instance(ReleaseId::prev(), code))
    }

    const fn instance(id: ReleaseId, code: Version) -> Self {
        Self { id, code }
    }

    pub fn release(self) -> ReleaseId {
        self.id
    }

    pub fn update_software(self, to: Self) -> Result<Self, StdError> {
        self.check_storage_match(to.code)
            .and_then(|()| self.allow_software_update_int(to))
    }

    pub fn update_software_and_storage(self, to: Self) -> Result<Self, StdError> {
        self.check_storage_adjacent(to.code)
            .and_then(|()| self.allow_software_update_int(to))
    }

    fn allow_software_update_int(&self, new: Self) -> Result<Self, StdError> {
        let current = self.code;
        if current < new.code || (self.id == ReleaseId::dev() && current == new.code) {
            Ok(new)
        } else {
            Err(StdError::generic_err(
                "The software version does not increase monotonically!",
            ))
        }
    }

    fn check_storage_match(&self, other: Version) -> Result<(), StdError> {
        if self.code.same_storage(&other) {
            Ok(())
        } else {
            Err(StdError::generic_err(format!(
                "The storage versions do not match! The new software version is \"{other}\"!",
            )))
        }
    }

    fn check_storage_adjacent(&self, next: Version) -> Result<(), StdError> {
        if self.code.next_storage(&next) {
            Ok(())
        } else {
            Err(StdError::generic_err(format!(
                "The new version \"{next}\" is not adjacent to the current one \"{current}\"!",
                current = self.code
            )))
        }
    }
}

impl From<ReleaseId> for String {
    fn from(value: ReleaseId) -> Self {
        value.0
    }
}

// TODO remove once the admin has completed the issue#466
impl From<PackageRelease> for ReleaseId {
    fn from(value: PackageRelease) -> Self {
        value.id
    }
}

#[cfg(test)]
mod test {
    use crate::{SemVer, Version};

    use super::{PackageRelease, ReleaseId};

    fn prod_id() -> ReleaseId {
        ReleaseId("v0.5.3".into())
    }

    #[test]
    fn prod_software() {
        let current = Version::new(1, SemVer::parse("0.3.4"));
        PackageRelease::instance(prod_id(), current)
            .update_software(PackageRelease::instance(prod_id(), current))
            .unwrap_err();
        PackageRelease::instance(prod_id(), current)
            .update_software(PackageRelease::instance(
                prod_id(),
                Version::new(current.storage + 1, current.software),
            ))
            .unwrap_err();

        PackageRelease::instance(prod_id(), current)
            .update_software(PackageRelease::instance(
                prod_id(),
                Version::new(current.storage, SemVer::parse("0.3.3")),
            ))
            .unwrap_err();

        let next_code = Version::new(current.storage, SemVer::parse("0.3.5"));
        assert_eq!(
            Ok(PackageRelease::instance(prod_id(), next_code)),
            PackageRelease::instance(prod_id(), current)
                .update_software(PackageRelease::instance(prod_id(), next_code,))
        );
    }

    #[test]
    fn dev_software() {
        let current = Version::new(1, SemVer::parse("0.3.4"));

        assert_eq!(
            Ok(PackageRelease::instance(ReleaseId::dev(), current)),
            PackageRelease::instance(ReleaseId::dev(), current)
                .update_software(PackageRelease::instance(ReleaseId::dev(), current))
        );
        PackageRelease::instance(ReleaseId::dev(), current)
            .update_software(PackageRelease::instance(
                ReleaseId::dev(),
                Version::new(current.storage + 1, SemVer::parse("0.3.4")),
            ))
            .unwrap_err();

        PackageRelease::instance(ReleaseId::dev(), current)
            .update_software(PackageRelease::instance(
                ReleaseId::dev(),
                Version::new(current.storage, SemVer::parse("0.3.3")),
            ))
            .unwrap_err();

        let next_code = Version::new(current.storage, SemVer::parse("0.3.5"));
        assert_eq!(
            Ok(PackageRelease::instance(ReleaseId::dev(), next_code)),
            PackageRelease::instance(ReleaseId::dev(), current)
                .update_software(PackageRelease::instance(ReleaseId::dev(), next_code))
        );
    }

    #[test]
    fn prod_software_and_storage() {
        let current = Version::new(1, SemVer::parse("0.3.4"));

        PackageRelease::instance(prod_id(), current)
            .update_software_and_storage(PackageRelease::instance(prod_id(), current))
            .unwrap_err();

        PackageRelease::instance(prod_id(), current)
            .update_software_and_storage(PackageRelease::instance(
                prod_id(),
                Version::new(current.storage + 1, SemVer::parse("0.3.3")),
            ))
            .unwrap_err();
        PackageRelease::instance(prod_id(), current)
            .update_software_and_storage(PackageRelease::instance(
                prod_id(),
                Version::new(current.storage + 1, current.software),
            ))
            .unwrap_err();

        let next_code = Version::new(current.storage + 1, SemVer::parse("0.3.5"));
        assert_eq!(
            Ok(PackageRelease::instance(prod_id(), next_code)),
            PackageRelease::instance(prod_id(), current)
                .update_software_and_storage(PackageRelease::instance(prod_id(), next_code))
        );

        PackageRelease::instance(prod_id(), current)
            .update_software_and_storage(PackageRelease::instance(
                prod_id(),
                Version::new(current.storage, SemVer::parse("0.3.5")),
            ))
            .unwrap_err();
    }

    #[test]
    fn dev_software_and_storage() {
        let current = Version::new(1, SemVer::parse("0.3.4"));

        PackageRelease::instance(ReleaseId::dev(), current)
            .update_software_and_storage(PackageRelease::instance(ReleaseId::dev(), current))
            .unwrap_err();

        PackageRelease::instance(ReleaseId::dev(), current)
            .update_software_and_storage(PackageRelease::instance(
                ReleaseId::dev(),
                Version::new(current.storage + 1, SemVer::parse("0.3.3")),
            ))
            .unwrap_err();
        let next_code = Version::new(current.storage + 1, SemVer::parse("0.3.5"));
        assert_eq!(
            Ok(PackageRelease::instance(ReleaseId::dev(), next_code)),
            PackageRelease::instance(ReleaseId::dev(), current)
                .update_software_and_storage(PackageRelease::instance(ReleaseId::dev(), next_code))
        );

        PackageRelease::instance(ReleaseId::dev(), current)
            .update_software_and_storage(PackageRelease::instance(
                ReleaseId::dev(),
                Version::new(current.storage, SemVer::parse("0.3.5")),
            ))
            .unwrap_err();
    }
}
