use std::error::Error;

use serde::{Deserialize, Serialize};

use sdk::{
    cosmwasm_std::{StdError, StdResult, Storage},
    cw_storage_plus::Item,
};

pub type VersionSegment = u16;

#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Serialize, Deserialize)]
pub struct SemVer {
    major: VersionSegment,
    minor: VersionSegment,
    patch: VersionSegment,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct Version {
    storage: VersionSegment,
    software: SemVer,
}

impl Version {
    pub fn new(storage: VersionSegment, software: SemVer) -> Self {
        Self { storage, software }
    }
}

pub fn parse_semver(version: &str) -> SemVer {
    fn parse_segment<'r, I>(
        iter: &mut I,
        lowercase_name: &str,
        pascal_case_name: &str,
    ) -> VersionSegment
    where
        I: Iterator<Item = &'r str> + ?Sized,
    {
        iter.next()
            .unwrap_or_else(|| panic!("No {} segment in version string!", lowercase_name))
            .parse()
            .unwrap_or_else(|_| {
                panic!(
                    "{} segment in version string is not a number!",
                    pascal_case_name
                )
            })
    }

    let mut iter = version.split('.');

    let major: VersionSegment = parse_segment(&mut iter, "major", "Major");
    let minor: VersionSegment = parse_segment(&mut iter, "minor", "Minor");
    let patch: VersionSegment = parse_segment(&mut iter, "patch", "Patch");

    if iter.next().is_some() {
        panic!("Unexpected fourth segment found in version string!");
    };

    SemVer {
        major,
        minor,
        patch,
    }
}

#[macro_export]
macro_rules! version {
    ($storage: expr) => {{
        $crate::Version::new(
            $storage,
            $crate::parse_semver(::core::env!(
                "CARGO_PKG_VERSION",
                "Cargo package version is not set as an environment variable!",
            )),
        )
    }};
}

const VERSION_STORAGE_KEY: Item<'static, Version> = Item::new("contract_version");

pub fn initialize(storage: &mut dyn Storage, version: Version) -> StdResult<()> {
    VERSION_STORAGE_KEY.save(storage, &version)
}

// TODO remove when all contracts have been migrated to post-refactor versions
pub fn upgrade_old_contract<
    'r,
    const OLD_COMPATIBILITY_VERSION: VersionSegment,
    MigrateStorageFunctor,
    MigrateStorageError,
>(
    storage: &'r mut dyn Storage,
    version: Version,
    migrate_storage_functor: Option<MigrateStorageFunctor>,
) -> Result<(), MigrateStorageError>
where
    MigrateStorageFunctor: FnOnce(&'r mut dyn Storage) -> Result<(), MigrateStorageError>,
    MigrateStorageError: From<StdError> + Error,
{
    const CW_VERSION_ITEM: Item<'static, String> = Item::new("contract_info");

    const OLD_VERSION_ITEM: Item<'static, u16> = Item::new("contract_version");

    if version.storage != 0 {
        return Err(StdError::generic_err(
            "Storage version should be set to zero, marking the initial one!",
        )
        .into());
    }

    if OLD_VERSION_ITEM.load(storage)? != OLD_COMPATIBILITY_VERSION {
        return Err(StdError::generic_err(
            "Couldn't upgrade contract because storage version didn't match expected one!",
        )
        .into());
    }

    CW_VERSION_ITEM.remove(storage);

    OLD_VERSION_ITEM.remove(storage);

    // Using zero as a starting storage version to mark this as a new epoch.
    initialize(storage, version)?;

    migrate_storage_functor.map_or(Ok(()), move |functor| functor(storage))
}

pub fn update_software(storage: &mut dyn Storage, version: Version) -> StdResult<()> {
    VERSION_STORAGE_KEY.update(storage, |saved_version| {
        if saved_version.storage != version.software {
            return Err(StdError::generic_err(format!("Software update handler called, but storage versions differ! Saved storage version is {saved}, but storage version used by this software is {current}!", saved = saved_version.storage, current = version.storage)));
        }

        if saved_version.software < version.software {
            Ok(version)
        } else {
            Err(StdError::generic_err(
                "Couldn't upgrade contract because version isn't monotonically increasing!",
            ))
        }
    })?;

    Ok(())
}

pub fn update_software_and_storage<
    'r,
    const FROM_STORAGE_VERSION: VersionSegment,
    MigrateStorageFunctor,
    MigrateStorageError,
>(
    storage: &'r mut dyn Storage,
    version: Version,
    migrate_storage: MigrateStorageFunctor,
) -> Result<(), MigrateStorageError>
where
    MigrateStorageFunctor: FnOnce(&'r mut dyn Storage) -> Result<(), MigrateStorageError>,
    MigrateStorageError: From<StdError> + Error,
{
    if version.storage == FROM_STORAGE_VERSION {
        return Err(StdError::generic_err("Software and storage update handler called, but expected and new storage versions are the same!").into());
    }

    if version.storage != FROM_STORAGE_VERSION.wrapping_add(1) {
        return Err(StdError::generic_err("Expected and new storage versions are not directly adjacent! This could indicate an error!").into());
    }

    VERSION_STORAGE_KEY.update(storage, |saved_version| {
        if saved_version.storage != version.storage {
            return Err(StdError::generic_err(
                "Couldn't upgrade contract because saved storage version didn't match expected one!",
            ));
        }

        if saved_version.software < component_version {
            Ok(version)
        } else {
            Err(StdError::generic_err(
                "Couldn't upgrade contract because software version isn't monotonically increasing!",
            ))
        }
    })?;

    migrate_storage(storage)
}
