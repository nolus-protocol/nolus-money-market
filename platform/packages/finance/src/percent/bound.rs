use std::fmt::{Debug, Display, Formatter, Result as FmtResult, Write};

#[cfg(any(test, feature = "testing"))]
use std::ops::{Add, Sub};

use sdk::cosmwasm_std::Uint256;
use serde::{Deserialize, Serialize};

use crate::{
    coin::Amount,
    error::Error,
    fraction::Unit as FractionUnit,
    fractionable::Fractionable,
    ratio::{Ratio, SimpleFraction},
    rational::Rational,
    zero::Zero,
};

use super::Units;

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(into = "Units", try_from = "Units")]
pub struct BoundPercent<const UPPER_BOUND: Units>(Units);

impl<const UPPER_BOUND: Units> BoundPercent<UPPER_BOUND> {
    pub const ZERO: Self = Self::try_from_primitive(0).expect("0% is a valid instance");
    pub const HUNDRED: Self = Self::try_from_primitive(100).expect("100% is a valid instance");
    pub(crate) const PRECISION: Self =
        Self::try_from_permille(1).expect("0.1% is a valid instance");

    const UNITS_TO_PERCENT_RATIO: Units = 10;

    #[cfg(any(test, feature = "testing"))]
    pub const fn from_percent(percent: u16) -> Self {
        Self::try_from_primitive(percent as u32)
            .expect("Percent value safely fits in internal representation")
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

    pub fn from_fraction<U>(nominator: U, denominator: U) -> Option<Self>
    where
        Self: Fractionable<U>,
        U: FractionUnit,
    {
        SimpleFraction::new(nominator, denominator).of(Self::HUNDRED)
    }

    pub const fn units(&self) -> Units {
        self.0
    }

    // Cannot be const because const impl of PartialEq is not available.
    pub fn is_zero(&self) -> bool {
        self == &Self::ZERO
    }

    pub fn checked_add(self, other: Self) -> Option<Self> {
        self.0
            .checked_add(other.0)
            .and_then(Self::try_from_permille)
    }

    pub fn checked_sub(self, other: Self) -> Option<Self> {
        self.0
            .checked_sub(other.0)
            .and_then(Self::try_from_permille)
    }
}

// Used for serialization
impl<const UPPER_BOUND: Units> From<BoundPercent<UPPER_BOUND>> for Units {
    fn from(percent: BoundPercent<UPPER_BOUND>) -> Self {
        percent.0
    }
}

// Used during the deserialization and general construction from a permille unit,
// ensuring it does not exceed the UPPER_BOUND.
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

impl<const UPPER_BOUND: Units> Ratio<Units> for BoundPercent<UPPER_BOUND> {
    fn parts(&self) -> Units {
        self.units()
    }

    fn total(&self) -> Units {
        Self::HUNDRED.units()
    }
}

impl<const UPPER: Units> FractionUnit for BoundPercent<UPPER> where
    BoundPercent<UPPER>: Copy + Debug + Ord + Zero
{
}

impl<const UPPER_BOUND: Units> From<BoundPercent<UPPER_BOUND>> for Amount {
    fn from(percent: BoundPercent<UPPER_BOUND>) -> Self {
        Amount::from(percent.units())
    }
}

impl<const UPPER_BOUND: Units> From<BoundPercent<UPPER_BOUND>> for SimpleFraction<Amount> {
    fn from(percent: BoundPercent<UPPER_BOUND>) -> Self {
        Self::new(
            percent.0.into(),
            BoundPercent::<UPPER_BOUND>::HUNDRED.0.into(),
        )
    }
}

// TODO remove it once the multiplication is refactored
impl<const UPPER_BOUND: Units> From<BoundPercent<UPPER_BOUND>> for Uint256 {
    fn from(percent: BoundPercent<UPPER_BOUND>) -> Self {
        Amount::from(percent).into()
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
        Self(
            self.0
                .checked_add(rhs.0)
                .expect("attempt to add with overflow"),
        )
    }
}

#[cfg(any(test, feature = "testing"))]
impl<const UPPER_BOUND: Units> Sub for BoundPercent<UPPER_BOUND> {
    type Output = Self;

    #[track_caller]
    fn sub(self, rhs: Self) -> Self {
        Self(
            self.0
                .checked_sub(rhs.0)
                .expect("attempt to subtract with overflow"),
        )
    }
}

#[cfg(test)]
mod test {
    use crate::percent::{Percent, Percent100};

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
}
