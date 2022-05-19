use std::ops::{Add, Sub};

use cosmwasm_std::{Timestamp, Uint128};
use serde::{Serialize, Deserialize};

/// A more storage and compute optimal version of its counterpart in the std::time.
/// Designed to represent a timespan between cosmwasm_std::Timestamp-s.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default, Serialize, Deserialize)]
pub struct Duration(u64);
impl Duration {
    const MILLIS_IN_SECOND: u64 = 1000 * 1000 * 1000;
    pub const YEAR: Duration = Duration(365 * 24 * 60 * 60 * Duration::MILLIS_IN_SECOND);

    pub const fn from_nanos(nanos: u64) -> Self {
        Self(nanos)
    }
    pub fn from_secs(secs: u32) -> Self {
        Self::from_nanos(u64::from(secs) * Duration::MILLIS_IN_SECOND)
    }
    pub fn between(start: Timestamp, end: Timestamp) -> Self {
        debug_assert!(start <= end);
        Self(end.nanos() - start.nanos())
    }
    pub fn nanos(&self) -> u64 {
        self.0
    }
}

impl From<Duration> for u128 {
    fn from(d: Duration) -> Self {
        d.nanos().into()
    }
}

impl From<Duration> for Uint128 {
    fn from(d: Duration) -> Self {
        u128::from(d).into()
    }
}

impl TryFrom<u128> for Duration {
    type Error = <u64 as TryFrom<u128>>::Error;

    fn try_from(value: u128) -> Result<Self, Self::Error> {
        Ok(Duration::from_nanos(value.try_into()?))
    }
}

impl Add<Duration> for Timestamp {
    type Output = Timestamp;

    fn add(self, rhs: Duration) -> Self::Output {
        self.plus_nanos(rhs.nanos())
    }
}

impl Sub<Duration> for Duration {
    type Output = Duration;

    fn sub(self, rhs: Duration) -> Self::Output {
        Self::Output::from_nanos(self.nanos().add(rhs.nanos()))
    }
}
