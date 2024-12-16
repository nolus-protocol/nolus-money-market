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

impl Release {
    const RELEASE_LABEL: &'static str = env!(
        "RELEASE_VERSION",
        "No release label provided as an environment variable! Please set \
        \"RELEASE_VERSION\" environment variable!",
    );

    const DEV_RELEASE: &'static str = "dev-release";

    const VOID_RELEASE: &'static str = "void-release";

    pub fn void() -> Self {
        Self::instance(Self::VOID_RELEASE)
    }

    pub(crate) fn from_env() -> Self {
        Self::instance(Self::RELEASE_LABEL)
    }

    fn instance<L>(label: L) -> Self
    where
        L: Into<String>,
    {
        Self(label.into())
    }

    pub fn allow_software_update(&self, current: &Version, new: &Version) -> Result<(), StdError> {
        check_storage_match(
            current.storage,
            new.storage,
            "the new software storage version",
        )?;

        self.allow_software_update_int(current, new)
    }

    pub fn allow_software_and_storage_update<const FROM_STORAGE_VERSION: VersionSegment>(
        &self,
        current: &Version,
        new: &Version,
    ) -> Result<(), StdError> {
        check_storage_match(
            current.storage,
            FROM_STORAGE_VERSION,
            "the expected origin storage version",
        )?;

        if current.storage.wrapping_add(1) == new.storage {
            self.allow_software_update_int(current, new)
        } else {
            Err(StdError::generic_err(
                "The storage version is not adjacent to the current one!",
            ))
        }
    }

    fn allow_software_update_int(&self, current: &Version, new: &Version) -> Result<(), StdError> {
        if current.software < new.software
            || (self.0 == Self::DEV_RELEASE && current.software == new.software)
        {
            Ok(())
        } else {
            Err(StdError::generic_err(
                "The software version does not increase monotonically!",
            ))
        }
    }
}

fn check_storage_match(
    current: VersionSegment,
    reference: VersionSegment,
    reference_descr: &str,
) -> Result<(), StdError> {
    if current != reference {
        Err(StdError::generic_err(format!(
        "The storage versions differ! The current storage version is {current} whereas {reference_descr} is {reference}!",
    )))
    } else {
        Ok(())
    }
}

impl From<Release> for String {
    fn from(value: Release) -> Self {
        value.0
    }
}

#[cfg(test)]
mod test {
    use crate::{SemVer, Version};

    use super::Release;

    const PROD_RELEASE: &str = "v0.5.3";

    #[test]
    fn prod_software() {
        let current = Version::new(1, SemVer::parse("0.3.4"));
        let instance = Release::instance(PROD_RELEASE);
        instance
            .allow_software_update(&current, &current)
            .unwrap_err();
        instance
            .allow_software_update(
                &current,
                &Version::new(current.storage + 1, SemVer::parse("0.3.4")),
            )
            .unwrap_err();

        instance
            .allow_software_update(
                &current,
                &Version::new(current.storage, SemVer::parse("0.3.3")),
            )
            .unwrap_err();

        let new = Version::new(1, SemVer::parse("0.3.5"));
        instance.allow_software_update(&current, &new).unwrap();
    }

    #[test]
    fn dev_software() {
        let instance = Release::instance(Release::DEV_RELEASE);
        let current = Version::new(1, SemVer::parse("0.3.4"));
        instance.allow_software_update(&current, &current).unwrap();
        instance
            .allow_software_update(
                &current,
                &Version::new(current.storage + 1, SemVer::parse("0.3.4")),
            )
            .unwrap_err();

        instance
            .allow_software_update(
                &current,
                &Version::new(current.storage, SemVer::parse("0.3.3")),
            )
            .unwrap_err();

        let new = Version::new(current.storage, SemVer::parse("0.3.5"));
        instance.allow_software_update(&current, &new).unwrap();
    }

    #[test]
    fn prod_software_and_storage() {
        let instance = Release::instance(PROD_RELEASE);
        let current = Version::new(1, SemVer::parse("0.3.4"));
        instance
            .allow_software_and_storage_update::<0>(&current, &current)
            .unwrap_err();
        instance
            .allow_software_and_storage_update::<1>(&current, &current)
            .unwrap_err();

        instance
            .allow_software_and_storage_update::<0>(
                &current,
                &Version::new(2, SemVer::parse("0.3.4")),
            )
            .unwrap_err();

        instance
            .allow_software_and_storage_update::<1>(
                &current,
                &Version::new(2, SemVer::parse("0.3.4")),
            )
            .unwrap_err();
        instance
            .allow_software_and_storage_update::<1>(
                &current,
                &Version::new(2, SemVer::parse("0.3.5")),
            )
            .unwrap();
        instance
            .allow_software_and_storage_update::<2>(
                &current,
                &Version::new(2, SemVer::parse("0.3.4")),
            )
            .unwrap_err();

        instance
            .allow_software_and_storage_update::<1>(
                &current,
                &Version::new(2, SemVer::parse("0.3.3")),
            )
            .unwrap_err();

        instance
            .allow_software_and_storage_update::<1>(
                &current,
                &Version::new(1, SemVer::parse("0.3.5")),
            )
            .unwrap_err();

        let new = Version::new(2, SemVer::parse("0.3.5"));
        instance
            .allow_software_and_storage_update::<1>(&current, &new)
            .unwrap();
        instance
            .allow_software_and_storage_update::<2>(&Version::new(2, SemVer::parse("0.3.4")), &new)
            .unwrap_err();
    }

    #[test]
    fn dev_software_and_storage() {
        let instance = Release::instance(Release::DEV_RELEASE);
        let current = Version::new(1, SemVer::parse("0.3.4"));
        instance
            .allow_software_and_storage_update::<0>(&current, &current)
            .unwrap_err();
        instance
            .allow_software_and_storage_update::<1>(&current, &current)
            .unwrap_err();

        instance
            .allow_software_and_storage_update::<0>(
                &current,
                &Version::new(2, SemVer::parse("0.3.4")),
            )
            .unwrap_err();

        instance
            .allow_software_and_storage_update::<1>(
                &current,
                &Version::new(2, SemVer::parse("0.3.4")),
            )
            .unwrap();
        instance
            .allow_software_and_storage_update::<1>(
                &current,
                &Version::new(2, SemVer::parse("0.3.5")),
            )
            .unwrap();
        instance
            .allow_software_and_storage_update::<2>(
                &current,
                &Version::new(2, SemVer::parse("0.3.4")),
            )
            .unwrap_err();

        instance
            .allow_software_and_storage_update::<1>(
                &current,
                &Version::new(2, SemVer::parse("0.3.3")),
            )
            .unwrap_err();

        instance
            .allow_software_and_storage_update::<1>(
                &current,
                &Version::new(1, SemVer::parse("0.3.5")),
            )
            .unwrap_err();

        let new = Version::new(2, SemVer::parse("0.3.5"));
        instance
            .allow_software_and_storage_update::<1>(&current, &new)
            .unwrap();
        instance
            .allow_software_and_storage_update::<2>(&Version::new(2, SemVer::parse("0.3.4")), &new)
            .unwrap_err();
    }
}
