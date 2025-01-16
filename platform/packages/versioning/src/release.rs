use std::borrow::Cow;

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
// The two usecases, building the current release, and deserializing a release, call for `&'static str` and String, respectively.
// We use Cow since it is an enum of the two. We do not need to mutate it.
pub struct ReleaseId(Cow<'static, str>);

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

    const CURRENT: Self = Self(Cow::Borrowed(Self::ID));

    //TODO delete once deliver a version with contracts that take the current release as input
    const PREV: Self = Self(Cow::Borrowed("v0.7.6"));

    const VOID: Self = Self(Cow::Borrowed("void-release"));
}

impl PackageRelease {
    pub const fn void() -> Self {
        Self::instance(
            ReleaseId::VOID,
            const { Version::new(SemVer::parse("0.0.0"), 0) },
        )
    }

    pub(crate) const fn current(code: Version) -> Self {
        Self::instance(ReleaseId::CURRENT, code)
    }

    pub(crate) fn pull_prev(storage: &mut dyn Storage) -> Result<Self, StdError> {
        const VERSION_STORAGE_KEY: Item<Version> = Item::new("contract_version");

        VERSION_STORAGE_KEY
            .load(storage)
            .inspect(|_| VERSION_STORAGE_KEY.remove(storage))
            .map(|code| Self::instance(ReleaseId::PREV, code))
    }

    const fn instance(id: ReleaseId, code: Version) -> Self {
        Self { id, code }
    }

    pub fn release(self) -> ReleaseId {
        self.id
    }

    pub(crate) fn update_software(self, to: Self) -> Result<Self, StdError> {
        self.check_software_update_allowed(to, Self::check_storage_match)
    }

    pub(crate) fn update_software_and_storage(self, to: Self) -> Result<Self, StdError> {
        self.check_software_update_allowed(to, Self::check_storage_adjacent)
    }

    fn check_software_update_allowed<F>(
        &self,
        new: Self,
        storage_check: F,
    ) -> Result<Self, StdError>
    where
        F: FnOnce(&Self, Version) -> Result<(), StdError>,
    {
        storage_check(self, new.code).and_then(|()| {
            let current_software = self.code;
            let new_software = new.code;
            if current_software <= new_software {
                Ok(new)
            } else {
                Err(StdError::generic_err(
                    "The software version does not increase monotonically!",
                ))
            }
        })
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
        value.0.to_string()
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
        let current = Version::new(SemVer::parse("0.3.4"), 1);
        let instance = PackageRelease::instance(prod_id(), current);
        assert_eq!(
            Ok(instance.clone()),
            instance.clone().update_software(instance.clone())
        );
        instance
            .clone()
            .update_software(PackageRelease::instance(
                prod_id(),
                Version::new(current.version, current.storage + 1),
            ))
            .unwrap_err();

        instance
            .clone()
            .update_software(PackageRelease::instance(
                prod_id(),
                Version::new(SemVer::parse("0.3.3"), current.storage),
            ))
            .unwrap_err();

        let next_code = Version::new(SemVer::parse("0.3.5"), current.storage);
        assert_eq!(
            Ok(PackageRelease::instance(prod_id(), next_code)),
            instance.update_software(PackageRelease::instance(prod_id(), next_code,))
        );
    }

    #[test]
    fn prod_software_and_storage() {
        let current = Version::new(SemVer::parse("0.3.4"), 1);
        let instance = PackageRelease::instance(prod_id(), current);

        instance
            .clone()
            .update_software_and_storage(instance.clone())
            .unwrap_err();

        instance
            .clone()
            .update_software_and_storage(PackageRelease::instance(
                prod_id(),
                Version::new(SemVer::parse("0.3.3"), current.storage + 1),
            ))
            .unwrap_err();

        let next_code = Version::new(SemVer::parse("0.3.5"), current.storage + 1);
        assert_eq!(
            Ok(PackageRelease::instance(prod_id(), next_code)),
            instance
                .clone()
                .update_software_and_storage(PackageRelease::instance(prod_id(), next_code))
        );

        instance
            .clone()
            .update_software_and_storage(PackageRelease::instance(
                prod_id(),
                Version::new(SemVer::parse("0.3.5"), current.storage),
            ))
            .unwrap_err();
    }
}
