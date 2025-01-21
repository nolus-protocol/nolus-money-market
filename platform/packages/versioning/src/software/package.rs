use std::{
    cmp::Ordering,
    fmt::{Display, Formatter, Result as FmtResult},
};

use serde::{Deserialize, Serialize};

#[cfg(feature = "schema")]
use sdk::schemars::{self, JsonSchema};

use super::version::{SemVer, VersionSegment};

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
/// A 'reference type' representing a software package
pub struct Package {
    /// the reference identification attribute
    version: SemVer,
    // TODO add `name: &str`, the Cargo package name
    storage: VersionSegment,
}

impl Package {
    pub const fn new(version: SemVer, storage: VersionSegment) -> Self {
        Self { version, storage }
    }

    pub const fn same_storage(&self, other: &Self) -> bool {
        self.check_storage(other.storage)
    }

    pub const fn next_storage(&self, other: &Self) -> bool {
        other.check_storage(self.storage.wrapping_add(1))
    }

    const fn check_storage(&self, expected: VersionSegment) -> bool {
        self.storage == expected
    }
}

impl Display for Package {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        f.write_fmt(format_args!(
            "version: {}, storage: {}",
            self.version, self.storage
        ))
    }
}

impl Eq for Package {}

impl PartialEq for Package {
    fn eq(&self, other: &Self) -> bool {
        let res = self.version == other.version;
        if res {
            debug_assert_eq!(self.storage, other.storage);
        }
        res
    }
}

impl PartialOrd for Package {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        let res = self.version.partial_cmp(&other.version);
        debug_assert_eq!(res == Some(Ordering::Equal), self.eq(other));
        res
    }
}
