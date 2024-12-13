use serde::{Deserialize, Serialize};

use sdk::{
    cosmwasm_std::StdError,
    schemars::{self, JsonSchema},
};

use super::{Version, VersionSegment};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema)]
#[repr(transparent)]
#[serde(transparent)]
pub struct Release(String);
//TODO merge ReleaseLabel with Type and refactor the free functions into member functions of the new type Release

impl Release {
    const RELEASE_LABEL: &'static str = env!(
        "RELEASE_VERSION",
        "No release label provided as an environment variable! Please set \
        \"RELEASE_VERSION\" environment variable!",
    );

    const DEV_RELEASE: &'static str = "dev-release";

    const VOID_RELEASE: &'static str = "void-release";

    pub fn void() -> Self {
        Self(Self::VOID_RELEASE.into())
    }

    pub(crate) fn label() -> Self {
        Self(Self::RELEASE_LABEL.into())
    }
}

impl From<Release> for String {
    fn from(value: Release) -> Self {
        value.0
    }
}

pub fn allow_software_update(current: &Version, new: &Version) -> Result<(), StdError> {
    allow_software_update_type(release_type(), current, new)
}

pub fn allow_software_and_storage_update<const FROM_STORAGE_VERSION: VersionSegment>(
    current: &Version,
    new: &Version,
) -> Result<(), StdError> {
    allow_software_and_storage_update_type::<FROM_STORAGE_VERSION>(release_type(), current, new)
}

fn allow_software_update_type(
    release_type: Type,
    current: &Version,
    new: &Version,
) -> Result<(), StdError> {
    if current.storage != new.storage {
        return Err(StdError::generic_err(format!(
            "The storage versions differ! The current storage version is {saved} whereas the storage version of the new software is {current}!",
            saved = current.storage,
            current = new.storage,
        )));
    }

    allow_software_update_int(release_type, current, new)
}

fn allow_software_and_storage_update_type<const FROM_STORAGE_VERSION: VersionSegment>(
    release_type: Type,
    current: &Version,
    new: &Version,
) -> Result<(), StdError> {
    if current.storage != FROM_STORAGE_VERSION {
        return Err(StdError::generic_err(format!(
            "The current storage version, {saved}, should match the expected one, {expected}!",
            saved = current.storage,
            expected = FROM_STORAGE_VERSION
        )));
    }

    if current.storage.wrapping_add(1) == new.storage {
        allow_software_update_int(release_type, current, new)
    } else {
        Err(StdError::generic_err(
            "The storage version is not adjacent to the current one!",
        ))
    }
}

fn allow_software_update_int(
    release_type: Type,
    current: &Version,
    new: &Version,
) -> Result<(), StdError> {
    if current.software < new.software
        || (release_type == Type::Dev && current.software == new.software)
    {
        Ok(())
    } else {
        Err(StdError::generic_err(
            "The software version does not increase monotonically!",
        ))
    }
}

#[derive(PartialEq, Eq)]
enum Type {
    Dev,
    Prod,
}

fn release_type() -> Type {
    if Release::RELEASE_LABEL == Release::DEV_RELEASE {
        Type::Dev
    } else {
        Type::Prod
    }
}

#[cfg(test)]
mod test {
    use crate::{SemVer, Version};

    use super::{allow_software_and_storage_update_type, allow_software_update_type, Type};

    #[test]
    fn prod_software() {
        let current = Version::new(1, SemVer::parse("0.3.4"));
        allow_software_update_type(Type::Prod, &current, &current).unwrap_err();
        allow_software_update_type(
            Type::Prod,
            &current,
            &Version::new(current.storage + 1, SemVer::parse("0.3.4")),
        )
        .unwrap_err();

        allow_software_update_type(
            Type::Prod,
            &current,
            &Version::new(current.storage, SemVer::parse("0.3.3")),
        )
        .unwrap_err();

        let new = Version::new(1, SemVer::parse("0.3.5"));
        allow_software_update_type(Type::Prod, &current, &new).unwrap();
    }

    #[test]
    fn dev_software() {
        let current = Version::new(1, SemVer::parse("0.3.4"));
        allow_software_update_type(Type::Dev, &current, &current).unwrap();
        allow_software_update_type(
            Type::Prod,
            &current,
            &Version::new(current.storage + 1, SemVer::parse("0.3.4")),
        )
        .unwrap_err();

        allow_software_update_type(
            Type::Prod,
            &current,
            &Version::new(current.storage, SemVer::parse("0.3.3")),
        )
        .unwrap_err();

        let new = Version::new(1, SemVer::parse("0.3.5"));
        allow_software_update_type(Type::Prod, &current, &new).unwrap();
    }

    #[test]
    fn prod_software_and_storage() {
        let current = Version::new(1, SemVer::parse("0.3.4"));
        allow_software_and_storage_update_type::<0>(Type::Prod, &current, &current).unwrap_err();
        allow_software_and_storage_update_type::<1>(Type::Prod, &current, &current).unwrap_err();

        allow_software_and_storage_update_type::<0>(
            Type::Prod,
            &current,
            &Version::new(2, SemVer::parse("0.3.4")),
        )
        .unwrap_err();

        allow_software_and_storage_update_type::<1>(
            Type::Prod,
            &current,
            &Version::new(2, SemVer::parse("0.3.4")),
        )
        .unwrap_err();
        allow_software_and_storage_update_type::<1>(
            Type::Prod,
            &current,
            &Version::new(2, SemVer::parse("0.3.5")),
        )
        .unwrap();
        allow_software_and_storage_update_type::<2>(
            Type::Prod,
            &current,
            &Version::new(2, SemVer::parse("0.3.4")),
        )
        .unwrap_err();

        allow_software_and_storage_update_type::<1>(
            Type::Prod,
            &current,
            &Version::new(2, SemVer::parse("0.3.3")),
        )
        .unwrap_err();

        allow_software_and_storage_update_type::<1>(
            Type::Prod,
            &current,
            &Version::new(1, SemVer::parse("0.3.5")),
        )
        .unwrap_err();

        let new = Version::new(2, SemVer::parse("0.3.5"));
        allow_software_and_storage_update_type::<1>(Type::Prod, &current, &new).unwrap();
        allow_software_and_storage_update_type::<2>(
            Type::Prod,
            &Version::new(2, SemVer::parse("0.3.4")),
            &new,
        )
        .unwrap_err();
    }

    #[test]
    fn dev_software_and_storage() {
        let current = Version::new(1, SemVer::parse("0.3.4"));
        allow_software_and_storage_update_type::<0>(Type::Dev, &current, &current).unwrap_err();
        allow_software_and_storage_update_type::<1>(Type::Dev, &current, &current).unwrap_err();

        allow_software_and_storage_update_type::<0>(
            Type::Dev,
            &current,
            &Version::new(2, SemVer::parse("0.3.4")),
        )
        .unwrap_err();

        allow_software_and_storage_update_type::<1>(
            Type::Dev,
            &current,
            &Version::new(2, SemVer::parse("0.3.4")),
        )
        .unwrap();
        allow_software_and_storage_update_type::<1>(
            Type::Dev,
            &current,
            &Version::new(2, SemVer::parse("0.3.5")),
        )
        .unwrap();
        allow_software_and_storage_update_type::<2>(
            Type::Dev,
            &current,
            &Version::new(2, SemVer::parse("0.3.4")),
        )
        .unwrap_err();

        allow_software_and_storage_update_type::<1>(
            Type::Dev,
            &current,
            &Version::new(2, SemVer::parse("0.3.3")),
        )
        .unwrap_err();

        allow_software_and_storage_update_type::<1>(
            Type::Dev,
            &current,
            &Version::new(1, SemVer::parse("0.3.5")),
        )
        .unwrap_err();

        let new = Version::new(2, SemVer::parse("0.3.5"));
        allow_software_and_storage_update_type::<1>(Type::Dev, &current, &new).unwrap();
        allow_software_and_storage_update_type::<2>(
            Type::Dev,
            &Version::new(2, SemVer::parse("0.3.4")),
            &new,
        )
        .unwrap_err();
    }
}
