use std::{
    fmt::Debug,
    ops::{Add, Div, Mul, Sub},
};

use cosmwasm_std::{Fraction, Timestamp, Uint128};
use serde::{Deserialize, Serialize};

use crate::{
    percentable::{Integer, TimeSliceable},
    ratio::Ratio,
};

pub type Units = u64;

/// A more storage and compute optimal version of its counterpart in the std::time.
/// Designed to represent a timespan between cosmwasm_std::Timestamp-s.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default, Serialize, Deserialize)]
pub struct Duration(Units);
impl Duration {
    const UNITS_IN_SECOND: Units = 1000 * 1000 * 1000;
    pub const YEAR: Duration = Duration::from_nanos(365 * 24 * 60 * 60 * Duration::UNITS_IN_SECOND);

    pub const fn from_nanos(nanos: Units) -> Self {
        Self(nanos)
    }
    pub fn from_secs(secs: u32) -> Self {
        Self::from_nanos(Units::from(secs) * Duration::UNITS_IN_SECOND)
    }
    pub fn between(start: Timestamp, end: Timestamp) -> Self {
        debug_assert!(start <= end);
        Self(end.nanos() - start.nanos())
    }
    pub const fn nanos(&self) -> Units {
        self.0
    }

    pub fn annualized_slice_of<T>(&self, annual_amount: T) -> T
    where
        T: TimeSliceable,
    {
        annual_amount.safe_mul(&Ratio::from(*self))
    }

    pub fn into_slice_per_ratio<U, D>(self, amount: U, annual_amount: U) -> Self
    where
        U: Default + PartialEq + Copy,
        Units: Integer<DoubleInteger = D> + TryFrom<D>,
        D: From<Units> + From<U> + Mul<D, Output = D> + Div<D, Output = D>,
        <Units as TryFrom<D>>::Error: Debug,
    {
        use crate::percentable::Fractionable;
        self.safe_mul(&Ratio::new(amount, annual_amount))
    }
}

impl From<Duration> for Ratio<Units> {
    fn from(p: Duration) -> Self {
        Self::new(p.nanos(), Duration::YEAR.nanos())
    }
}

struct DurationPerYear {
    nominator: Duration,
}

impl Fraction<Units> for DurationPerYear {
    fn numerator(&self) -> Units {
        self.nominator.nanos()
    }

    fn denominator(&self) -> Units {
        Duration::YEAR.nanos()
    }

    fn inv(&self) -> Option<Self> {
        todo!();
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
    type Error = <Units as TryFrom<u128>>::Error;

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
