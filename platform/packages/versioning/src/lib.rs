use serde::{Deserialize, Serialize};

#[cfg(feature = "schema")]
use sdk::schemars::{self, JsonSchema};
use sdk::{
    cosmwasm_std::{StdError, StdResult, Storage},
    cw_storage_plus::Item,
};

use self::release::ReleaseLabel;

mod release;

pub type VersionSegment = u16;

#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub struct SemVer {
    major: VersionSegment,
    minor: VersionSegment,
    patch: VersionSegment,
}

impl SemVer {
    pub const fn parse(version: &str) -> Self {
        let version: &[u8] = version.as_bytes();
        let mut version_index: usize = 0;

        let mut segments: [VersionSegment; 3] = [0; 3];
        let mut segment_index: usize = 0;

        while version_index < version.len() {
            match version[version_index] {
                digit @ b'0'..=b'9' => {
                    segments[segment_index] *= 10;
                    segments[segment_index] += (digit - b'0') as VersionSegment;
                }
                b'.' => {
                    segment_index += 1;

                    if segment_index == segments.len() {
                        panic!("Unexpected segment!");
                    } else if version_index + 1 == version.len() {
                        panic!("Version can't end with a dot!");
                    }
                }
                _ => unreachable!(),
            }

            version_index += 1;
        }

        if segment_index < segments.len() - 1 {
            unreachable!()
        }

        let [major, minor, patch]: [VersionSegment; 3] = segments;

        Self {
            major,
            minor,
            patch,
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub struct Version {
    storage: VersionSegment,
    software: SemVer,
}

impl Version {
    pub const fn new(storage: VersionSegment, software: SemVer) -> Self {
        Self { storage, software }
    }
}

#[macro_export]
macro_rules! package_version {
    () => {{
        $crate::SemVer::parse(::core::env!(
            "CARGO_PKG_VERSION",
            "Cargo package version is not set as an environment variable!",
        ))
    }};
}

#[macro_export]
macro_rules! version {
    ($storage: expr) => {{
        $crate::Version::new($storage, $crate::package_version!())
    }};
    ($storage: expr, $version: expr) => {{
        $crate::Version::new($storage, $version)
    }};
}

const VERSION_STORAGE_KEY: Item<'static, Version> = Item::new("contract_version");

pub fn initialize(storage: &mut dyn Storage, version: Version) -> StdResult<()> {
    VERSION_STORAGE_KEY.save(storage, &version)
}

pub fn update_software<ContractError, MapErrorFunctor>(
    storage: &mut dyn Storage,
    new: Version,
    map_error: MapErrorFunctor,
) -> Result<ReleaseLabel, ContractError>
where
    MapErrorFunctor: FnOnce(StdError) -> ContractError,
{
    load_version(storage)
        .and_then(|current| release::allow_software_update(&current, &new))
        .and_then(|()| save_version(storage, &new))
        .map(|()| release::label())
        .map_err(map_error)
}

pub fn update_software_and_storage<
    const FROM_STORAGE_VERSION: VersionSegment,
    ContractError,
    MigrateStorageFunctor,
    MigrateStorageFunctorResponse,
    MapErrorFunctor,
>(
    storage: &mut dyn Storage,
    new: Version,
    migrate_storage: MigrateStorageFunctor,
    map_error: MapErrorFunctor,
) -> Result<(ReleaseLabel, MigrateStorageFunctorResponse), ContractError>
where
    MigrateStorageFunctor:
        FnOnce(&mut dyn Storage) -> Result<MigrateStorageFunctorResponse, ContractError>,
    MapErrorFunctor: FnOnce(StdError) -> ContractError,
{
    load_version(storage)
        .and_then(|current| {
            release::allow_software_and_storage_update::<FROM_STORAGE_VERSION>(&current, &new)
        })
        .and_then(|()| save_version(storage, &new))
        .map_err(map_error)
        .and_then(|()| migrate_storage(storage))
        .map(|response| (release::label(), response))
}

fn load_version(storage: &mut dyn Storage) -> Result<Version, StdError> {
    VERSION_STORAGE_KEY.load(storage)
}

fn save_version(storage: &mut dyn Storage, new: &Version) -> Result<(), StdError> {
    VERSION_STORAGE_KEY.save(storage, new)
}
