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
macro_rules! package_version {
    () => {{
        $crate::parse_semver(::core::env!(
            "CARGO_PKG_VERSION",
            "Cargo package version is not set as an environment variable!",
        ))
    }};
}

const VERSION_ITEM: Item<'static, Version> = Item::new("contract_version");

pub fn initialize<const STORAGE_VERSION: VersionSegment>(
    storage: &mut dyn Storage,
    component_version: SemVer,
) -> StdResult<()> {
    VERSION_ITEM.save(
        storage,
        &Version {
            storage: STORAGE_VERSION,
            software: component_version,
        },
    )
}

// TODO remove when all contracts have been migrated to post-refactor versions
pub fn upgrade_old_contract<
    'r,
    const OLD_COMPATIBILITY_VERSION: VersionSegment,
    MigrateStorageFunctor,
    MigrateStorageError,
>(
    storage: &'r mut dyn Storage,
    component_version: SemVer,
    migrate_storage_functor: Option<MigrateStorageFunctor>,
) -> Result<(), MigrateStorageError>
where
    MigrateStorageFunctor: FnOnce(&'r mut dyn Storage) -> Result<(), MigrateStorageError>,
    MigrateStorageError: From<StdError> + Error,
{
    const CW_VERSION_ITEM: Item<'static, String> = Item::new("contract_info");

    const OLD_VERSION_ITEM: Item<'static, u16> = Item::new("contract_version");

    if OLD_VERSION_ITEM.load(storage)? != OLD_COMPATIBILITY_VERSION {
        return Err(StdError::generic_err(
            "Couldn't upgrade contract because storage version didn't match expected one!",
        )
        .into());
    }

    CW_VERSION_ITEM.remove(storage);

    OLD_VERSION_ITEM.remove(storage);

    // Using zero as a starting storage version to mark this as a new epoch.
    initialize::<0>(storage, component_version)?;

    migrate_storage_functor.map_or(Ok(()), move |functor| functor(storage))
}

pub fn update_software<const CURRENT_STORAGE_VERSION: VersionSegment>(
    storage: &mut dyn Storage,
    component_version: SemVer,
) -> StdResult<()> {
    VERSION_ITEM.update(storage, |mut version_pair| {
        if version_pair.storage != CURRENT_STORAGE_VERSION {
            return Err(StdError::generic_err(format!("Software update handler called, but storage versions differ! Saved storage version is {saved}, but storage version used by this software is {current}!", saved = version_pair.storage, current = CURRENT_STORAGE_VERSION)));
        }

        if version_pair.software < component_version {
            version_pair.software = component_version;

            Ok(version_pair)
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
    const NEW_STORAGE_VERSION: VersionSegment,
    MigrateStorageFunctor,
    MigrateStorageError,
>(
    storage: &'r mut dyn Storage,
    component_version: SemVer,
    migrate_storage: MigrateStorageFunctor,
) -> Result<(), MigrateStorageError>
where
    MigrateStorageFunctor: FnOnce(&'r mut dyn Storage) -> Result<(), MigrateStorageError>,
    MigrateStorageError: From<StdError> + Error,
{
    if FROM_STORAGE_VERSION == NEW_STORAGE_VERSION {
        return Err(StdError::generic_err("Software and storage update handler called, but expected and new storage versions are the same!").into());
    }

    if FROM_STORAGE_VERSION.wrapping_add(1) != NEW_STORAGE_VERSION {
        return Err(StdError::generic_err("Expected and new storage versions are not directly adjacent! This could indicate an error!").into());
    }

    VERSION_ITEM.update(storage, |version_pair| {
        if version_pair.storage != FROM_STORAGE_VERSION {
            return Err(StdError::generic_err(
                "Couldn't upgrade contract because saved storage version didn't match expected one!",
            ));
        }

        if version_pair.software < component_version {
            Ok(Version {storage: NEW_STORAGE_VERSION, software: component_version})
        } else {
            Err(StdError::generic_err(
                "Couldn't upgrade contract because software version isn't monotonically increasing!",
            ))
        }
    })?;

    migrate_storage(storage)
}
