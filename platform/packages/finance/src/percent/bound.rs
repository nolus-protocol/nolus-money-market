use std::fmt::{Debug, Display, Formatter, Result as FmtResult, Write};

#[cfg(any(test, feature = "testing"))]
use std::ops::{Add, Sub};

use serde::{Deserialize, Serialize};

use crate::{
    
    coin::{Amount,
    DoubleCoinPrimitive},
    error::Error,
    fraction::Unit as FractionUnit,
   
    ratio::{RatioLegacy, SimpleFraction},
,
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

    pub(crate) fn to_fraction<U>(self) -> SimpleFraction<U>
    where
        U: FractionUnit + From<Self>,
    {
        SimpleFraction::new(self.into(), Self::HUNDRED.into())
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

// TODO: Revisit it's usage after refactoring Fractionable
impl<const UPPER_BOUND: Units> From<BoundPercent<UPPER_BOUND>> for u128 {
    fn from(percent: BoundPercent<UPPER_BOUND>) -> Self {
        Amount::from(percent.0)
    }
}

// TODO: Remove when Fractionable trait boundaries include the traits ToPrimitive and TryFromPrimitive
impl<const UPPER_BOUND: Units> From<BoundPercent<UPPER_BOUND>> for DoubleCoinPrimitive {
    fn from(percent: BoundPercent<UPPER_BOUND>) -> Self {
        percent.units().into()
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
    use crate::{
        fraction::Fraction,
        percent::{Percent, Percent100, Units, test},
        ratio::SimpleFraction,
        rational::Rational,
        test::coin,
    };

    #[test]
    fn from_primitive() {
        assert_eq!(Percent100::try_from_primitive(0).unwrap(), Percent100::ZERO);
        assert_eq!(
            Percent100::try_from_primitive(10).unwrap(),
            test::percent100(100)
        );
        assert_eq!(
            Percent100::try_from_primitive(99).unwrap(),
            test::percent100(990)
        );
        assert_eq!(
            Percent100::try_from_primitive(100).unwrap(),
            test::percent100(1000)
        );
        assert!(Percent100::try_from_primitive(101).is_none());

        assert_eq!(Percent::try_from_primitive(0).unwrap(), Percent::ZERO);
        assert_eq!(
            Percent::try_from_primitive(101).unwrap(),
            test::percent(1010)
        );
    }

    #[test]
    fn from_permille() {
        assert_eq!(Percent100::try_from_permille(0).unwrap(), Percent100::ZERO);
        assert_eq!(
            Percent100::try_from_permille(10).unwrap(),
            test::percent100(10)
        );
        assert_eq!(
            Percent100::try_from_permille(1000).unwrap(),
            test::percent100(1000)
        );

        assert_eq!(Percent::try_from_permille(0).unwrap(), Percent::ZERO);
        assert_eq!(
            Percent::try_from_permille(1001).unwrap(),
            test::percent(1001)
        );
        assert!(Percent::try_from_primitive(u32::MAX / 10 + 1).is_none());
    }

    #[test]
    fn test_zero() {
        let zero_amount = coin::coin1(0);
        assert_eq!(zero_amount, Percent100::ZERO.of(coin::coin1(10)));
        assert_eq!(zero_amount, Percent::ZERO.of(coin::coin1(10)).unwrap())
    }

    #[test]
    fn test_hundred() {
        let amount = coin::coin1(123);
        assert_eq!(amount, Percent100::HUNDRED.of(amount));
        assert_eq!(amount, Percent::HUNDRED.of(amount).unwrap())
    }

    #[test]
    fn checked_add() {
        assert_eq!(
            test::percent100(40),
            test::percent100(25) + (test::percent100(15))
        );
        assert_eq!(
            test::percent100(39),
            test::percent100(0) + (test::percent100(39))
        );
        assert_eq!(
            test::percent100(39),
            test::percent100(39) + (test::percent100(0))
        );
        assert_eq!(
            Percent100::HUNDRED,
            test::percent100(999) + (test::percent100(1))
        );
    }

    #[test]
    fn add_overflow() {
        assert!(
            Percent100::HUNDRED
                .checked_add(test::percent100(1))
                .is_none()
        );
        assert!(
            Percent::from_permille(Units::MAX)
                .checked_add(Percent::from_permille(1))
                .is_none()
        );
    }

    #[test]
    fn sub() {
        assert_eq!(
            test::percent100(67),
            test::percent100(79) - (test::percent100(12))
        );
        assert_eq!(
            test::percent100(0),
            test::percent100(34) - (test::percent100(34))
        );
        assert_eq!(
            test::percent100(39),
            test::percent100(39) - (test::percent100(0))
        );
        assert_eq!(test::percent100(990), test::percent100(10).complement());
        assert_eq!(test::percent100(0), test::percent100(1000).complement());
    }

    #[test]
    fn sub_overflow() {
        assert!(
            test::percent100(34)
                .checked_sub(test::percent100(35))
                .is_none()
        )
    }

    #[test]
    fn to_fraction() {
        assert_eq!(
            SimpleFraction::new(Percent100::ZERO, Percent100::HUNDRED),
            Percent100::ZERO.to_fraction()
        );
        assert_eq!(
            SimpleFraction::new(Percent::HUNDRED, Percent::HUNDRED),
            Percent::HUNDRED.to_fraction()
        );
        assert_eq!(
            SimpleFraction::new(test::percent(1001), Percent::HUNDRED),
            test::percent(1001).to_fraction()
        );
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

    fn test_display(exp: &str, permilles: Units) {
        assert_eq!(exp, format!("{}", test::percent100(permilles)));
    }
}
