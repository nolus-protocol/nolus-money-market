use serde::{Deserialize, Serialize};

use sdk::{
    cosmwasm_ext::as_dyn::{storage, AsDyn},
    cosmwasm_std::{StdError, StdResult},
    cw_storage_plus::Item,
    schemars::{self, JsonSchema},
};

pub use self::release::ReleaseLabel;

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

                    assert!(segment_index != segments.len(), "Unexpected segment!");
                    assert!(
                        version_index + 1 < version.len(),
                        "Version can't end with a dot!"
                    );
                }
                _ => panic!(
                    "Unexpected symbol encountered! Expected an ASCII number or an ASCII dot!"
                ),
            }

            version_index += 1;
        }

        assert!(segment_index + 1 == segments.len(), "Invalid version string! Expected three segments (major, minor and patch), but got less!");

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
            "Cargo package version is not set as an environment variable!"
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

pub fn initialize<S>(storage: &mut S, version: Version) -> StdResult<()>
where
    S: storage::DynMut + ?Sized,
{
    VERSION_STORAGE_KEY.save(storage.as_dyn_mut(), &version)
}

pub fn update_software<S, ContractError, MapErrorFunctor>(
    storage: &mut S,
    new: Version,
    map_error: MapErrorFunctor,
) -> Result<ReleaseLabel, ContractError>
where
    S: storage::DynMut + ?Sized,
    MapErrorFunctor: FnOnce(StdError) -> ContractError,
{
    load_version(storage)
        .and_then(|current| release::allow_software_update(&current, &new))
        .and_then(|()| save_version(storage, &new))
        .map(|()| release::label())
        .map_err(map_error)
}

pub fn update_software_and_storage<
    S,
    const FROM_STORAGE_VERSION: VersionSegment,
    ContractError,
    MigrateStorageFunctor,
    MigrateStorageFunctorResponse,
    MapErrorFunctor,
>(
    storage: &mut S,
    new: Version,
    migrate_storage: MigrateStorageFunctor,
    map_error: MapErrorFunctor,
) -> Result<(ReleaseLabel, MigrateStorageFunctorResponse), ContractError>
where
    S: storage::DynMut + ?Sized,
    MigrateStorageFunctor: FnOnce(&mut S) -> Result<MigrateStorageFunctorResponse, ContractError>,
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

fn load_version<S>(storage: &mut S) -> Result<Version, StdError>
where
    S: storage::Dyn + ?Sized,
{
    VERSION_STORAGE_KEY.load(storage.as_dyn())
}

fn save_version<S>(storage: &mut S, new: &Version) -> Result<(), StdError>
where
    S: storage::DynMut + ?Sized,
{
    VERSION_STORAGE_KEY.save(storage.as_dyn_mut(), new)
}

#[cfg(test)]
mod tests {
    use crate::SemVer;

    #[test]
    fn valid() {
        const VERSIONS: &[(&str, SemVer)] = &[
            (
                "0.0.1",
                SemVer {
                    major: 0,
                    minor: 0,
                    patch: 1,
                },
            ),
            (
                "1.3.2",
                SemVer {
                    major: 1,
                    minor: 3,
                    patch: 2,
                },
            ),
            (
                "12.34.56",
                SemVer {
                    major: 12,
                    minor: 34,
                    patch: 56,
                },
            ),
        ];

        for &(version, expected) in VERSIONS {
            assert_eq!(SemVer::parse(version), expected);
        }
    }

    #[test]
    #[should_panic = "Invalid version string! Expected three segments (major, minor and patch), but got less!"]
    fn invalid_empty() {
        _ = SemVer::parse("");
    }

    #[test]
    #[should_panic = "Invalid version string! Expected three segments (major, minor and patch), but got less!"]
    fn invalid_one_segment() {
        _ = SemVer::parse("1");
    }

    #[test]
    #[should_panic = "Version can't end with a dot!"]
    fn invalid_one_segment_and_dot() {
        _ = SemVer::parse("1.");
    }

    #[test]
    #[should_panic = "Invalid version string! Expected three segments (major, minor and patch), but got less!"]
    fn invalid_two_segments() {
        _ = SemVer::parse("1.2");
    }

    #[test]
    #[should_panic = "Version can't end with a dot!"]
    fn invalid_two_segments_and_dot() {
        _ = SemVer::parse("1.2.");
    }

    #[test]
    #[should_panic = "Unexpected segment!"]
    fn invalid_three_segments_and_dot() {
        _ = SemVer::parse("1.2.3.");
    }

    #[test]
    #[should_panic = "Unexpected segment!"]
    fn invalid_four_segments() {
        _ = SemVer::parse("1.2.3.4");
    }

    #[test]
    #[should_panic = "Unexpected symbol encountered! Expected an ASCII number or an ASCII dot!"]
    fn excluded_postfix() {
        _ = SemVer::parse("1.2.3-rc1");
    }
}
