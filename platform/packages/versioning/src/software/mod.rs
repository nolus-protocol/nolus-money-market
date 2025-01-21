use std::borrow::Cow;

use serde::{Deserialize, Serialize};

use sdk::{
    cosmwasm_std::{StdError, Storage},
    cw_storage_plus::Item,
    schemars::{self, JsonSchema},
};

use package::Package;
pub use version::{SemVer, VersionSegment};

mod package;
mod version;

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
    code: Package,
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

    pub const VOID: Self = Self(Cow::Borrowed("void-release"));
}

impl PackageRelease {
    pub const fn current(name: &'static str, version: &str, storage: VersionSegment) -> Self {
        Self::instance(
            ReleaseId::CURRENT,
            Package::new(name, SemVer::parse(version), storage),
        )
    }

    pub(crate) fn pull_prev(
        name: &'static str,
        storage: &mut dyn Storage,
    ) -> Result<Self, StdError> {
        #[derive(Deserialize)]
        pub struct LegacyPackage {
            storage: VersionSegment,
            software: SemVer,
        }

        const VERSION_STORAGE_KEY: Item<LegacyPackage> = Item::new("contract_version");

        impl LegacyPackage {
            fn migrate_to(self, name: &'static str) -> Package {
                Package::new(name, self.software, self.storage)
            }
        }
        impl Serialize for LegacyPackage {
            fn serialize<S>(&self, _: S) -> Result<S::Ok, S::Error>
            where
                S: serde::Serializer,
            {
                unimplemented!("LegacyPackage is not meant to be serialized")
            }
        }

        VERSION_STORAGE_KEY
            .load(storage)
            .inspect(|_| VERSION_STORAGE_KEY.remove(storage))
            .map(|code| Self::instance(ReleaseId::PREV, code.migrate_to(name)))
    }

    const fn instance(id: ReleaseId, code: Package) -> Self {
        Self { id, code }
    }

    pub fn release(self) -> ReleaseId {
        self.id
    }

    pub const fn version(&self) -> SemVer {
        self.code.version()
    }

    pub(crate) fn update_software(self, to: Self) -> Result<Self, StdError> {
        self.check_software_update_allowed(to, Self::check_storage_match)
    }

    pub(crate) fn update_software_and_storage(self, to: Self) -> Result<Self, StdError> {
        self.check_software_update_allowed(to, Self::check_storage_adjacent)
    }

    fn check_software_update_allowed<F>(&self, to: Self, storage_check: F) -> Result<Self, StdError>
    where
        F: FnOnce(&Self, &Package) -> Result<(), StdError>,
    {
        self.check_name_match(&to.code)
            .and_then(|()| self.check_code_same_or_newer(&to.code))
            .and_then(|()| storage_check(self, &to.code))
            .map(|()| to)
    }

    fn check_name_match(&self, other: &Package) -> Result<(), StdError> {
        if self.code.same_name(other) {
            Ok(())
        } else {
            Err(StdError::generic_err(format!(
                "The package names do not match! The new package is \"{other}\".",
            )))
        }
    }
    fn check_code_same_or_newer(&self, other: &Package) -> Result<(), StdError> {
        if &self.code <= other {
            Ok(())
        } else {
            Err(StdError::generic_err(format!(
                "The software version does not increase monotonically! The new package is \"{other}\"."
            )))
        }
    }

    fn check_storage_match(&self, other: &Package) -> Result<(), StdError> {
        if self.code.same_storage(other) {
            Ok(())
        } else {
            Err(StdError::generic_err(format!(
                "The storage versions do not match! The new package is \"{other}\".",
            )))
        }
    }

    fn check_storage_adjacent(&self, next: &Package) -> Result<(), StdError> {
        if self.code.next_storage(next) {
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

    use super::{version::VersionSegment, Package, PackageRelease, ReleaseId, SemVer};

    const CURRENT_NAME: &str = "package_A";
    const CURRENT_VERSION: SemVer = SemVer::parse("0.3.4");
    const CURRENT_STORAGE: VersionSegment = 1;

    const OTHER_NAME: &str = "package_B";
    const NEWER_VERSION: SemVer = SemVer::parse("0.3.5");

    fn prod_id() -> ReleaseId {
        ReleaseId("v0.5.3".into())
    }

    #[test]
    fn prod_software() {
        let current = Package::new(CURRENT_NAME, CURRENT_VERSION, CURRENT_STORAGE);
        let instance = PackageRelease::instance(prod_id(), current);
        assert_eq!(
            Ok(instance.clone()),
            instance.clone().update_software(instance.clone())
        );
        instance
            .clone()
            .update_software(PackageRelease::instance(
                prod_id(),
                Package::new(OTHER_NAME, CURRENT_VERSION, CURRENT_STORAGE),
            ))
            .unwrap_err();

        instance
            .clone()
            .update_software(PackageRelease::instance(
                prod_id(),
                Package::new(CURRENT_NAME, SemVer::parse("0.3.3"), CURRENT_STORAGE),
            ))
            .unwrap_err();

        let next_code = Package::new(CURRENT_NAME, NEWER_VERSION, CURRENT_STORAGE);
        assert_eq!(
            Ok(PackageRelease::instance(prod_id(), next_code.clone())),
            instance
                .clone()
                .update_software(PackageRelease::instance(prod_id(), next_code,))
        );

        instance
            .clone()
            .update_software(PackageRelease::instance(
                prod_id(),
                Package::new(OTHER_NAME, NEWER_VERSION, CURRENT_STORAGE),
            ))
            .unwrap_err();
    }

    #[test]
    fn prod_software_and_storage() {
        let current = Package::new(CURRENT_NAME, CURRENT_VERSION, CURRENT_STORAGE);
        let instance = PackageRelease::instance(prod_id(), current);

        instance
            .clone()
            .update_software_and_storage(instance.clone())
            .unwrap_err();

        instance
            .clone()
            .update_software_and_storage(PackageRelease::instance(
                prod_id(),
                Package::new(OTHER_NAME, CURRENT_VERSION, CURRENT_STORAGE),
            ))
            .unwrap_err();

        instance
            .clone()
            .update_software_and_storage(PackageRelease::instance(
                prod_id(),
                Package::new(CURRENT_NAME, SemVer::parse("0.3.3"), CURRENT_STORAGE + 1),
            ))
            .unwrap_err();

        let next_code = Package::new(CURRENT_NAME, NEWER_VERSION, CURRENT_STORAGE + 1);
        assert_eq!(
            Ok(PackageRelease::instance(prod_id(), next_code.clone())),
            instance
                .clone()
                .update_software_and_storage(PackageRelease::instance(prod_id(), next_code))
        );

        instance
            .clone()
            .update_software_and_storage(PackageRelease::instance(
                prod_id(),
                Package::new(OTHER_NAME, NEWER_VERSION, CURRENT_STORAGE + 1),
            ))
            .unwrap_err();

        instance
            .clone()
            .update_software_and_storage(PackageRelease::instance(
                prod_id(),
                Package::new(CURRENT_NAME, NEWER_VERSION, CURRENT_STORAGE),
            ))
            .unwrap_err();
    }
}
