use serde::{Deserialize, Serialize};

#[cfg(feature = "schema")]
use sdk::schemars::{self, JsonSchema};
use sdk::{
    cosmwasm_std::{StdError, StdResult, Storage},
    cw_storage_plus::Item,
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

// TODO [Edition=2024] Remove `#[expect]`.
#[expect(edition_2024_expr_fragment_specifier)]
#[macro_export]
macro_rules! version {
    ($storage: expr) => {{
        $crate::Version::new($storage, $crate::package_version!())
    }};
    ($storage: expr, $version: expr) => {{
        $crate::Version::new($storage, $version)
    }};
}

const VERSION_STORAGE_KEY: Item<Version> = Item::new("contract_version");

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
        .map(|()| ReleaseLabel::label())
        .map_err(map_error)
}

pub struct FullUpdateOutput<MigrateStorageOutput> {
    pub release_label: ReleaseLabel,
    pub storage_migration_output: MigrateStorageOutput,
}

pub fn update_software_and_storage<
    const FROM_STORAGE_VERSION: VersionSegment,
    ContractError,
    MigrateStorageFunctor,
    StorageMigrationOutput,
    MapErrorFunctor,
>(
    storage: &mut dyn Storage,
    new: Version,
    migrate_storage: MigrateStorageFunctor,
    map_error: MapErrorFunctor,
) -> Result<FullUpdateOutput<StorageMigrationOutput>, ContractError>
where
    MigrateStorageFunctor:
        FnOnce(&mut dyn Storage) -> Result<StorageMigrationOutput, ContractError>,
    MapErrorFunctor: FnOnce(StdError) -> ContractError,
{
    load_version(storage)
        .and_then(|current| {
            release::allow_software_and_storage_update::<FROM_STORAGE_VERSION>(&current, &new)
        })
        .and_then(|()| save_version(storage, &new))
        .map_err(map_error)
        .and_then(|()| migrate_storage(storage))
        .map(|storage_migration_output| FullUpdateOutput {
            release_label: ReleaseLabel::label(),
            storage_migration_output,
        })
}

fn load_version(storage: &mut dyn Storage) -> Result<Version, StdError> {
    VERSION_STORAGE_KEY.load(storage)
}

fn save_version(storage: &mut dyn Storage, new: &Version) -> Result<(), StdError> {
    VERSION_STORAGE_KEY.save(storage, new)
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
