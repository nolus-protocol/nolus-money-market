use std::{
    fmt::Debug,
    ops::{Add, Sub},
};

use cosmwasm_std::{Timestamp, Uint128};
use serde::{Deserialize, Serialize};

use crate::{
    fraction::Fraction,
    fractionable::{Fractionable, TimeSliceable},
    ratio::Rational,
};

pub type Units = u64;

/// A more storage and compute optimal version of its counterpart in the std::time.
/// Designed to represent a timespan between cosmwasm_std::Timestamp-s.
///
/// Implementation note: We use `as` safely for numeric upcasts instead of `from/into`
/// in order to get const result.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default, Serialize, Deserialize)]
pub struct Duration(Units);
impl Duration {
    const UNITS_IN_SECOND: Units = 1000 * 1000 * 1000;
    const UNITS_IN_DAY: Units = Self::UNITS_IN_SECOND * Self::SECONDS_IN_DAY as Units;

    const SECONDS_IN_MINUTE: u32 = 60;
    const SECONDS_IN_HOUR: u32 = Self::SECONDS_IN_MINUTE * Self::MINUTES_IN_HOUR as u32;
    const SECONDS_IN_DAY: u32 = Self::SECONDS_IN_HOUR * Self::HOURS_IN_DAY as u32;

    const MINUTES_IN_HOUR: u16 = 60;
    const HOURS_IN_DAY: u16 = 24;

    pub const YEAR: Duration = Self::from_days(365);

    pub const fn from_nanos(nanos: Units) -> Self {
        Self(nanos)
    }

    pub const fn from_secs(secs: u32) -> Self {
        Self::from_nanos(secs as Units * Self::UNITS_IN_SECOND)
    }

    pub const fn from_minutes(minutes: u16) -> Self {
        Self::from_secs(minutes as u32 * Self::SECONDS_IN_MINUTE)
    }

    pub const fn from_hours(hours: u16) -> Self {
        Self::from_secs(hours as u32 * Self::SECONDS_IN_HOUR)
    }

    pub const fn from_days(days: u16) -> Self {
        Self::from_nanos(days as Units * Self::UNITS_IN_DAY)
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
        annual_amount.safe_mul(&Rational::new(self.nanos(), Self::YEAR.nanos()))
    }

    pub fn into_slice_per_ratio<U>(self, amount: U, annual_amount: U) -> Self
    where
        Self: Fractionable<U>,
        U: Default + PartialEq + Copy,
    {
        Rational::new(amount, annual_amount).of(self)
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
        Ok(Self::from_nanos(value.try_into()?))
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
        Self::Output::from_nanos(self.nanos().sub(rhs.nanos()))
    }
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::Timestamp as T;

    use crate::duration::{Duration as D, Units};

    #[test]
    fn add() {
        let t = T::from_seconds(100);
        assert_eq!(T::from_seconds(t.seconds() + 10), t + D::from_secs(10));
        assert_eq!(T::from_nanos(t.nanos() + 1), t + D::from_nanos(1));
        assert_eq!(t, t + D::from_secs(0));
        assert_eq!(
            T::from_nanos(u64::MAX),
            T::from_nanos(u64::MAX - 12) + D::from_nanos(12)
        );
    }

    #[test]
    #[should_panic]
    fn add_overflow() {
        let _ = T::from_nanos(u64::MAX - 12) + D::from_nanos(13);
    }

    #[test]
    fn sub() {
        let d = D::from_secs(12345678);
        assert_eq!(D::from_nanos(0), d - d);
    }

    #[test]
    #[should_panic]
    fn sub_underflow() {
        let _ = D::from_nanos(0) - D::from_nanos(1);
    }

    #[test]
    fn between() {
        let d = D::from_secs(422);
        let t1 = T::from_seconds(24);
        let t2 = t1 + d;

        assert_eq!(d, D::between(t1, t2));
    }

    #[test]
    #[should_panic]
    fn between_underflow() {
        let t = T::from_seconds(24);
        let _ = D::between(t + D::from_nanos(1), t);
    }

    #[test]
    fn from_max() {
        assert_eq!(
            D::between(T::from_nanos(0), T::from_nanos(Units::MAX)),
            D::from_nanos(Units::MAX)
        );
        assert_eq!(
            D::from_nanos(Units::from(u32::MAX) * D::UNITS_IN_SECOND),
            D::from_secs(u32::MAX)
        );
        assert_eq!(
            D::from_secs(u32::from(u16::MAX) * D::SECONDS_IN_MINUTE),
            D::from_minutes(u16::MAX)
        );
        assert_eq!(
            D::from_secs(u32::from(u16::MAX) * D::SECONDS_IN_HOUR),
            D::from_hours(u16::MAX)
        );
        assert_eq!(
            D::from_nanos(
                Units::from(u16::MAX) * D::UNITS_IN_SECOND * Units::from(D::SECONDS_IN_DAY)
            ),
            D::from_days(u16::MAX)
        );
    }

    #[test]
    fn constants() {
        assert_eq!(D::from_secs(1), D::from_nanos(D::UNITS_IN_SECOND));
        assert_eq!(D::from_minutes(1), D::from_secs(D::SECONDS_IN_MINUTE));
        assert_eq!(D::from_hours(1), D::from_minutes(D::MINUTES_IN_HOUR));
        assert_eq!(D::from_days(1), D::from_hours(D::HOURS_IN_DAY));
    }

    #[test]
    fn year() {
        assert_eq!(D::from_days(365), D::YEAR);
    }
}
