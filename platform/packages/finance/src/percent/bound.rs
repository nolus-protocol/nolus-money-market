use std::{
    fmt::{Debug, Display, Formatter, Result as FmtResult, Write},
    ops::Rem,
};

#[cfg(any(test, feature = "testing"))]
use std::ops::{Add, Sub};

use bnum::types::U256;
use serde::{Deserialize, Serialize};

use crate::{
    coin::Amount, error::Error, fraction::Unit as FractionUnit, ratio::RatioLegacy, zero::Zero,
};

use super::Units;

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(into = "Units", try_from = "Units")]
pub struct BoundPercent<const UPPER_BOUND: Units>(Units);

impl<const UPPER_BOUND: Units> BoundPercent<UPPER_BOUND> {
    pub const ZERO: Self = Self::try_from_primitive(0).expect("0% is a valid instance");
    pub const HUNDRED: Self = Self::try_from_primitive(100).expect("100% is a valid instance");
    pub const PRECISION: Self = Self::try_from_permille(1).expect("0.1% is a valid instance");

    const UNITS_TO_PERCENT_RATIO: Units = 10;

    #[cfg(any(test, feature = "testing"))]
    pub const fn from_percent(percent: u32) -> Self {
        Self::try_from_primitive(percent).expect("Percent value exceeds allowed upper bound")
    }

    #[cfg(any(test, feature = "testing"))]
    pub const fn from_permille(permille: Units) -> Self {
        Self::try_from_permille(permille).expect("Permille value exceeds allowed upper bound")
    }

    const fn try_from_primitive(percent: u32) -> Option<Self> {
        if let Some(permille) = percent.checked_mul(Self::UNITS_TO_PERCENT_RATIO) {
            Self::try_from_permille(permille)
        } else {
            None
        }
    }

    const fn try_from_permille(permille: Units) -> Option<Self> {
        if permille <= UPPER_BOUND {
            Some(Self(permille))
        } else {
            None
        }
    }

    // TODO revisit it's usage and remove
    pub const fn units(&self) -> Units {
        self.0
    }

    // Cannot be const because const impl of PartialEq is not available.
    pub fn is_zero(&self) -> bool {
        self == &Self::ZERO
    }

    pub const fn checked_add(self, other: Self) -> Option<Self> {
        if let Some(res) = self.0.checked_add(other.0) {
            Self::try_from_permille(res)
        } else {
            None
        }
    }

    pub const fn checked_sub(self, other: Self) -> Option<Self> {
        if let Some(res) = self.0.checked_sub(other.0) {
            Self::try_from_permille(res)
        } else {
            None
        }
    }
}

impl<const UPPER_BOUND: Units> From<BoundPercent<UPPER_BOUND>> for Units {
    fn from(percent: BoundPercent<UPPER_BOUND>) -> Self {
        percent.0
    }
}

impl<const UPPER_BOUND: Units> TryFrom<Units> for BoundPercent<UPPER_BOUND> {
    type Error = Error;

    fn try_from(permille: Units) -> Result<Self, Self::Error> {
        Self::try_from_permille(permille).ok_or(Error::UpperBoundCrossed {
            bound: UPPER_BOUND,
            value: permille,
        })
    }
}

impl<const UPPER_BOUND: Units> Display for BoundPercent<UPPER_BOUND> {
    #[track_caller]
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        let whole = (self.0) / Self::UNITS_TO_PERCENT_RATIO;
        let (no_fraction, overflow) = whole.overflowing_mul(Self::UNITS_TO_PERCENT_RATIO);
        debug_assert!(!overflow);
        let (fractional, overflow) = (self.0).overflowing_sub(no_fraction);
        debug_assert!(!overflow);

        f.write_fmt(format_args!("{whole}"))?;
        if fractional != Units::default() {
            f.write_fmt(format_args!(".{fractional}"))?;
        }
        f.write_char('%')?;
        Ok(())
    }
}

impl<const UPPER: Units> FractionUnit for BoundPercent<UPPER>
where
    BoundPercent<UPPER>: Copy + Debug + Ord + Zero,
{
    type Times = Units;

    fn gcd(self, other: Self) -> Self::Times {
        FractionUnit::gcd(self.units(), other.units())
    }

    fn scale_down(self, scale: Self::Times) -> Self {
        debug_assert_ne!(scale, Self::Times::ZERO);
        Self::try_from_permille(self.units().scale_down(scale))
            .expect("Scaled down Units are greater than UPPER_BOUND")
    }

    fn modulo(self, scale: Self::Times) -> Self::Times {
        self.units().rem(scale)
    }
}

// TODO: Revisit it's usage after refactoring Fractionable
impl<const UPPER_BOUND: Units> From<BoundPercent<UPPER_BOUND>> for u128 {
    fn from(percent: BoundPercent<UPPER_BOUND>) -> Self {
        Amount::from(percent.0)
    }
}

// TODO: Remove when Fractionable trait boundaries include the traits ToPrimitive and TryFromPrimitive
impl<const UPPER_BOUND: Units> From<BoundPercent<UPPER_BOUND>> for U256 {
    fn from(percent: BoundPercent<UPPER_BOUND>) -> Self {
        Amount::from(percent).into()
    }
}

impl<const UPPER_BOUND: Units> RatioLegacy<Units> for BoundPercent<UPPER_BOUND> {
    fn parts(&self) -> Units {
        self.0
    }

    fn total(&self) -> Units {
        Self::HUNDRED.0
    }
}

impl<const UPPER_BOUND: Units> Zero for BoundPercent<UPPER_BOUND> {
    const ZERO: Self = Self::ZERO;
}

#[cfg(any(test, feature = "testing"))]
impl<const UPPER_BOUND: Units> Add for BoundPercent<UPPER_BOUND> {
    type Output = Self;

    #[track_caller]
    fn add(self, rhs: Self) -> Self {
        self.checked_add(rhs).expect("attempt to add with overflow")
    }
}

#[cfg(any(test, feature = "testing"))]
impl<const UPPER_BOUND: Units> Sub for BoundPercent<UPPER_BOUND> {
    type Output = Self;

    #[track_caller]
    fn sub(self, rhs: Self) -> Self {
        self.checked_sub(rhs)
            .expect("attempt to subtract with overflow")
    }
}

#[cfg(test)]
mod test {
    use currency::test::SubGroupTestC10;

    use crate::{
        coin::Coin,
        fraction::Fraction,
        percent::{Percent, Percent100, Units},
        rational::Rational,
    };

    #[test]
    fn from_primitive() {
        assert_eq!(Percent100::try_from_primitive(0).unwrap(), Percent100::ZERO);
        assert_eq!(
            Percent100::try_from_primitive(10).unwrap(),
            Percent100::from_percent(10)
        );
        assert_eq!(
            Percent100::try_from_primitive(99).unwrap(),
            Percent100::from_percent(99)
        );
        assert_eq!(
            Percent100::try_from_primitive(100).unwrap(),
            Percent100::from_percent(100)
        );
        assert!(Percent100::try_from_primitive(101).is_none());

        assert_eq!(Percent::try_from_primitive(0).unwrap(), Percent::ZERO);
        assert_eq!(
            Percent::try_from_primitive(101).unwrap(),
            Percent::from_percent(101)
        );
    }

    #[test]
    fn from_permille() {
        assert_eq!(Percent100::try_from_permille(0).unwrap(), Percent100::ZERO);
        assert_eq!(
            Percent100::try_from_permille(10).unwrap(),
            Percent100::from_permille(10)
        );
        assert_eq!(
            Percent100::try_from_permille(1000).unwrap(),
            Percent100::from_permille(1000)
        );

        assert_eq!(Percent::try_from_permille(0).unwrap(), Percent::ZERO);
        assert_eq!(
            Percent::try_from_permille(1001).unwrap(),
            Percent::from_permille(1001)
        );
        assert!(Percent::try_from_primitive(u32::MAX / 10 + 1).is_none());
    }

    #[test]
    fn test_zero() {
        let zero_amount = Coin::<SubGroupTestC10>::new(0);
        assert_eq!(
            zero_amount,
            Percent100::ZERO.of(Coin::<SubGroupTestC10>::new(10))
        );
        assert_eq!(
            zero_amount,
            Percent::ZERO.of(Coin::<SubGroupTestC10>::new(10)).unwrap()
        )
    }

    #[test]
    fn test_hundred() {
        let amount = Coin::<SubGroupTestC10>::new(123);
        assert_eq!(amount, Percent100::HUNDRED.of(amount));
        assert_eq!(amount, Percent::HUNDRED.of(amount).unwrap())
    }

    #[test]
    fn checked_add() {
        assert_eq!(from(40), from(25) + (from(15)));
        assert_eq!(from(39), from(0) + (from(39)));
        assert_eq!(from(39), from(39) + (from(0)));
        assert_eq!(Percent100::HUNDRED, from(999) + (from(1)));
    }

    #[test]
    fn add_overflow() {
        assert!(Percent100::HUNDRED.checked_add(from(1)).is_none());
        assert!(
            Percent::from_permille(Units::MAX)
                .checked_add(Percent::from_permille(1))
                .is_none()
        );
    }

    #[test]
    fn sub() {
        assert_eq!(from(67), from(79) - (from(12)));
        assert_eq!(from(0), from(34) - (from(34)));
        assert_eq!(from(39), from(39) - (from(0)));
        assert_eq!(from(990), from(10).complement());
        assert_eq!(from(0), from(1000).complement());
    }

    #[test]
    fn sub_overflow() {
        assert!(from(34).checked_sub(from(35)).is_none())
    }

    #[test]
    fn display() {
        test_display("0%", 0);
        test_display("0.1%", 1);
        test_display("0.4%", 4);
        test_display("1%", 10);
        test_display("1.9%", 19);
        test_display("9%", 90);
        test_display("10.1%", 101);
        test_display("100%", 1000);
    }

    fn from(permille: Units) -> Percent100 {
        Percent100::from_permille(permille)
    }

    fn test_display(exp: &str, permilles: Units) {
        assert_eq!(exp, format!("{}", Percent100::from_permille(permilles)));
    }
}
