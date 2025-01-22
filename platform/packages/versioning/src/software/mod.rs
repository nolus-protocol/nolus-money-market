use std::borrow::Cow;

use serde::{Deserialize, Serialize};

use sdk::{
    cw_storage_plus::Item,
    schemars::{self, JsonSchema},
};

pub use self::{
    package::Package,
    version::{SemVer, VersionSegment},
};

use crate::{Error, UpdatablePackage};

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
    pub fn pull_prev(
        name: &'static str,
        storage: &mut dyn sdk::cosmwasm_std::Storage,
    ) -> Result<Self, Error> {
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
            .map_err(Error::loading)
            .inspect(|_| VERSION_STORAGE_KEY.remove(storage))
            .map(|code| Self::instance(ReleaseId::PREV, code.migrate_to(name)))
    }

    pub const fn current(name: &'static str, version: &str, storage: VersionSegment) -> Self {
        Self::instance(
            ReleaseId::CURRENT,
            Package::new(name, SemVer::parse(version), storage),
        )
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

    fn check_software_update_allowed<F>(&self, to: &Self, storage_check: F) -> Result<(), Error>
    where
        F: FnOnce(&Self, &Package) -> Result<(), Error>,
    {
        self.check_name_match(&to.code)
            .and_then(|()| self.check_code_same_or_newer(&to.code))
            .and_then(|()| storage_check(self, &to.code))
    }

    fn check_name_match(&self, other: &Package) -> Result<(), Error> {
        if self.code.same_name(other) {
            Ok(())
        } else {
            Err(Error::package_names_mismatch(&self.code, other))
        }
    }
    fn check_code_same_or_newer(&self, other: &Package) -> Result<(), Error> {
        if &self.code <= other {
            Ok(())
        } else {
            Err(Error::older_package_code(&self.code, other))
        }
    }

    fn check_storage_match(&self, other: &Package) -> Result<(), Error> {
        if self.code.same_storage(other) {
            Ok(())
        } else {
            Err(Error::package_storage_versions_mismatch(&self.code, other))
        }
    }

    fn check_storage_adjacent(&self, next: &Package) -> Result<(), Error> {
        if self.code.next_storage(next) {
            Ok(())
        } else {
            Err(Error::package_storage_version_not_adjacent(
                &self.code, next,
            ))
        }
    }
}

impl UpdatablePackage for PackageRelease {
    fn update_software(&self, to: &Self) -> Result<(), Error> {
        self.check_software_update_allowed(to, Self::check_storage_match)
    }

    fn update_software_and_storage(&self, to: &Self) -> Result<(), Error> {
        self.check_software_update_allowed(to, Self::check_storage_adjacent)
    }
}

impl From<ReleaseId> for String {
    fn from(value: ReleaseId) -> Self {
        value.0.to_string()
    }
}

#[cfg(test)]
mod test {

    use crate::UpdatablePackage;

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
        assert_eq!(Ok(()), instance.clone().update_software(&instance));
        instance
            .clone()
            .update_software(&PackageRelease::instance(
                prod_id(),
                Package::new(OTHER_NAME, CURRENT_VERSION, CURRENT_STORAGE),
            ))
            .unwrap_err();

        instance
            .clone()
            .update_software(&PackageRelease::instance(
                prod_id(),
                Package::new(CURRENT_NAME, SemVer::parse("0.3.3"), CURRENT_STORAGE),
            ))
            .unwrap_err();

        let next_code = Package::new(CURRENT_NAME, NEWER_VERSION, CURRENT_STORAGE);
        assert_eq!(
            Ok(()),
            instance
                .clone()
                .update_software(&PackageRelease::instance(prod_id(), next_code,))
        );

        instance
            .clone()
            .update_software(&PackageRelease::instance(
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
            .update_software_and_storage(&instance)
            .unwrap_err();

        instance
            .clone()
            .update_software_and_storage(&PackageRelease::instance(
                prod_id(),
                Package::new(OTHER_NAME, CURRENT_VERSION, CURRENT_STORAGE),
            ))
            .unwrap_err();

        instance
            .clone()
            .update_software_and_storage(&PackageRelease::instance(
                prod_id(),
                Package::new(CURRENT_NAME, SemVer::parse("0.3.3"), CURRENT_STORAGE + 1),
            ))
            .unwrap_err();

        let next_code = Package::new(CURRENT_NAME, NEWER_VERSION, CURRENT_STORAGE + 1);
        assert_eq!(
            Ok(()),
            instance
                .clone()
                .update_software_and_storage(&PackageRelease::instance(prod_id(), next_code))
        );

        instance
            .update_software_and_storage(&PackageRelease::instance(
                prod_id(),
                Package::new(OTHER_NAME, NEWER_VERSION, CURRENT_STORAGE + 1),
            ))
            .unwrap_err();

        instance
            .update_software_and_storage(&PackageRelease::instance(
                prod_id(),
                Package::new(CURRENT_NAME, NEWER_VERSION, CURRENT_STORAGE),
            ))
            .unwrap_err();
    }
}
