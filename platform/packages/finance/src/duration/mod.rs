use std::{
    fmt::{Debug, Display, Formatter, Result as FmtResult},
    ops::{Add, AddAssign, Sub, SubAssign},
};

use serde::{Deserialize, Serialize};

use sdk::cosmwasm_std::Timestamp;

use crate::{
    fraction::Unit as FractionUnit,
    fractionable::{CommonDoublePrimitive, Fractionable, IntoMax},
    ratio::SimpleFraction,
    rational::Rational,
};

mod fraction;
mod fractionable;

pub type Units = u64;

pub type Seconds = u32;

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

    const SECONDS_IN_MINUTE: Seconds = 60;
    const SECONDS_IN_HOUR: Seconds = Self::SECONDS_IN_MINUTE * Self::MINUTES_IN_HOUR as Seconds;
    const SECONDS_IN_DAY: Seconds = Self::SECONDS_IN_HOUR * Self::HOURS_IN_DAY as Seconds;

    const MINUTES_IN_HOUR: u16 = 60;
    const HOURS_IN_DAY: u16 = 24;

    pub const HOUR: Duration = Self::from_hours(1);

    pub const YEAR: Duration = Self::from_days(365);

    pub const MAX: Duration = Self::from_nanos(Units::MAX);

    pub const fn from_nanos(nanos: Units) -> Self {
        Self(nanos)
    }

    pub const fn from_secs(secs: Seconds) -> Self {
        Self::from_nanos(secs as Units * Self::UNITS_IN_SECOND)
    }

    pub const fn from_minutes(minutes: u16) -> Self {
        Self::from_secs(minutes as Seconds * Self::SECONDS_IN_MINUTE)
    }

    pub const fn from_hours(hours: u16) -> Self {
        Self::from_secs(hours as Seconds * Self::SECONDS_IN_HOUR)
    }

    pub const fn from_days(days: u16) -> Self {
        Self::from_nanos(days as Units * Self::UNITS_IN_DAY)
    }

    #[track_caller]
    pub fn between(start: &Timestamp, end: &Timestamp) -> Self {
        debug_assert!(start <= end);
        Self(end.nanos() - start.nanos())
    }

    pub const fn nanos(&self) -> Units {
        self.0
    }

    pub const fn micros(&self) -> Units {
        self.nanos() / 1000
    }

    pub const fn millis(&self) -> Units {
        self.micros() / 1000
    }

    pub const fn secs(&self) -> Units {
        self.millis() / 1000
    }

    pub fn checked_mul(&self, rhs: u16) -> Option<Self> {
        self.nanos().checked_mul(rhs.into()).map(Self::from_nanos)
    }

    #[track_caller]
    pub fn annualized_slice_of<T>(&self, annual_amount: T) -> Option<T>
    where
        Self: IntoMax<T::CommonDouble>,
        T: Fractionable<Self>,
    {
        SimpleFraction::new(*self, Self::YEAR).of(annual_amount)
    }

    pub fn into_slice_per_ratio<U>(self, amount: U, annual_amount: U) -> Option<Self>
    where
        Self: Fractionable<U>,
        U: FractionUnit + IntoMax<<Self as CommonDoublePrimitive<U>>::CommonDouble>,
    {
        SimpleFraction::new(amount, annual_amount).of(self)
    }
}

impl Add<Duration> for Timestamp {
    type Output = Self;

    #[track_caller]
    fn add(self, rhs: Duration) -> Self::Output {
        (&self).add(rhs)
    }
}

impl Add<Duration> for &Timestamp {
    type Output = Timestamp;

    #[track_caller]
    fn add(self, rhs: Duration) -> Self::Output {
        self.plus_nanos(rhs.nanos())
    }
}

impl AddAssign<Duration> for Timestamp {
    #[track_caller]
    fn add_assign(&mut self, rhs: Duration) {
        *self = self.add(rhs);
    }
}

impl Add<Duration> for Duration {
    type Output = Self;

    #[track_caller]
    fn add(self, rhs: Duration) -> Self::Output {
        Self::from_nanos(self.nanos().add(rhs.nanos()))
    }
}

impl Sub<Duration> for Timestamp {
    type Output = Self;

    #[track_caller]
    fn sub(self, rhs: Duration) -> Self::Output {
        (&self).sub(rhs)
    }
}

impl Sub<Duration> for &Timestamp {
    type Output = Timestamp;

    #[track_caller]
    fn sub(self, rhs: Duration) -> Self::Output {
        self.minus_nanos(rhs.nanos())
    }
}

impl SubAssign<Duration> for Timestamp {
    #[track_caller]
    fn sub_assign(&mut self, rhs: Duration) {
        *self = self.sub(rhs);
    }
}

impl Sub<Duration> for Duration {
    type Output = Self;

    #[track_caller]
    fn sub(self, rhs: Duration) -> Self::Output {
        Self::from_nanos(self.nanos().sub(rhs.nanos()))
    }
}

impl Display for Duration {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        f.write_fmt(format_args!("{} {}", self.nanos(), "nanos"))
    }
}

#[cfg(test)]
mod tests {
    use currency::test::SubGroupTestC10;
    use sdk::cosmwasm_std::Timestamp as T;

    use crate::{
        coin::{Amount, Coin},
        duration::{Duration as D, Seconds, Units},
        zero::Zero,
    };

    mod arithmetics {
        use super::{D, T, Units};

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
        fn add_asssign() {
            let mut t = T::from_seconds(100);
            t += D::from_secs(200);
            assert_eq!(T::from_seconds(300), t);
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
        fn sub_asssign() {
            let mut t = T::from_seconds(100);
            t -= D::from_secs(20);
            assert_eq!(T::from_seconds(80), t);
        }

        #[test]
        #[should_panic]
        fn sub_underflow() {
            let _ = D::from_nanos(0) - D::from_nanos(1);
        }

        #[test]
        fn checked_mul() {
            assert_eq!(Some(D::from_secs(10)), D::from_secs(5).checked_mul(2));
            assert_eq!(Some(D::from_secs(0)), D::from_secs(5).checked_mul(0));
        }

        #[test]
        fn checked_mul_overflow() {
            assert_eq!(None, D::from_nanos(Units::MAX).checked_mul(2));
            assert_eq!(
                None,
                D::from_nanos(Units::MAX / Units::from(u16::MAX) + 1).checked_mul(u16::MAX)
            );
        }
    }

    #[test]
    fn between() {
        let d = D::from_secs(422);
        let t1 = T::from_seconds(24);
        let t2 = t1 + d;

        assert_eq!(d, D::between(&t1, &t2));
    }

    #[test]
    #[should_panic]
    fn between_underflow() {
        let t = T::from_seconds(24);
        let _ = D::between(&(t + D::from_nanos(1)), &t);
    }

    #[test]
    fn from_max() {
        assert_eq!(
            D::between(&T::from_nanos(0), &T::from_nanos(Units::MAX)),
            D::from_nanos(Units::MAX)
        );
        assert_eq!(
            D::from_nanos(Units::from(Seconds::MAX) * D::UNITS_IN_SECOND),
            D::from_secs(Seconds::MAX)
        );
        assert_eq!(
            D::from_secs(Seconds::from(u16::MAX) * D::SECONDS_IN_MINUTE),
            D::from_minutes(u16::MAX)
        );
        assert_eq!(
            D::from_secs(Seconds::from(u16::MAX) * D::SECONDS_IN_HOUR),
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

    #[test]
    fn annualized_slice_of() {
        let annual_amount = test_coin(100000);
        assert_eq!(
            annual_amount,
            D::YEAR.annualized_slice_of(annual_amount).unwrap()
        );
        let expect_day_amount = annual_amount.checked_div(365).unwrap();
        assert_eq!(
            expect_day_amount,
            D::from_days(1).annualized_slice_of(annual_amount).unwrap()
        );
        let expect_hour_amount = expect_day_amount.checked_div(24).unwrap();
        assert_eq!(
            expect_hour_amount,
            D::HOUR.annualized_slice_of(annual_amount).unwrap()
        )
    }

    #[test]
    fn panic_annualized_slice_of() {
        assert!(
            (D::YEAR + D::HOUR)
                .annualized_slice_of(test_coin(Amount::MAX))
                .is_none()
        )
    }

    #[test]
    fn into_slice_per_ratio() {
        assert_eq!(
            D::from_days(365 / 5),
            D::YEAR
                .into_slice_per_ratio(test_coin(1), test_coin(5))
                .unwrap()
        );
        assert_eq!(
            D::from_days(10),
            D::from_days(30)
                .into_slice_per_ratio(test_coin(25), test_coin(75))
                .unwrap()
        );
        assert_eq!(
            D::ZERO,
            D::YEAR
                .into_slice_per_ratio(Coin::ZERO, test_coin(Amount::MAX))
                .unwrap()
        );
        assert_eq!(
            D::from_days(365 / 5),
            D::YEAR
                .into_slice_per_ratio(test_coin(Amount::MAX / 5), test_coin(Amount::MAX))
                .unwrap()
        );
    }

    #[test]
    fn panic_into_slice_per_ratio() {
        assert!(
            D::YEAR
                .into_slice_per_ratio(test_coin(585), test_coin(1))
                .is_none()
        )
    }

    const fn test_coin(amount: Amount) -> Coin<SubGroupTestC10> {
        Coin::new(amount)
    }
}
