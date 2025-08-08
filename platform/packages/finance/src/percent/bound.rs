use std::{
    fmt::{Debug, Display, Formatter, Result as FmtResult, Write},
    num::TryFromIntError,
    ops::{Div, Mul},
};

#[cfg(any(test, feature = "testing"))]
use std::ops::{Add, Sub};

use sdk::cosmwasm_std::Uint256;
use serde::{Deserialize, Serialize};

use crate::{
    coin::Amount,
    error::{Error, Result as FinanceResult},
    fraction::Unit as FractionUnit,
    fractionable::Fractionable,
    ratio::{Ratio, SimpleFraction},
    rational::Rational,
    zero::Zero,
};

use super::{HUNDRED_BOUND, Units};

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(try_from = "Units", into = "Units")]
pub struct BoundPercent<const UPPER_BOUND: Units>(Units);

impl<const UPPER_BOUND: Units> BoundPercent<UPPER_BOUND> {
    pub const ZERO: Self = Self::from_permille(0);
    pub const HUNDRED: Self = Self::from_permille(Self::PERMILLE);
    pub(crate) const PERMILLE: Units = HUNDRED_BOUND;

    const UNITS_TO_PERCENT_RATIO: Units = 10;

    #[cfg(any(test, feature = "testing"))]
    pub fn new(units: Units) -> Self {
        Self::new_internal(units)
    }

    const fn new_internal(units: Units) -> Self {
        debug_assert!(units <= UPPER_BOUND, "Value exceeds upper bound!");
        Self(units)
    }

    pub fn from_percent(percent: u16) -> Self {
        Self::from_permille(Units::from(percent) * Self::UNITS_TO_PERCENT_RATIO)
    }

    pub const fn from_permille(permille: Units) -> Self {
        Self::new_internal(permille)
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

    pub fn is_zero(&self) -> bool {
        self == &Self::ZERO
    }

    pub fn checked_add(self, other: Self) -> FinanceResult<Self> {
        self.0
            .checked_add(other.0)
            .ok_or(Error::overflow_err("while adding", self, other))
            .and_then(|sum| {
                if sum <= UPPER_BOUND {
                    Ok(Self::from_permille(sum))
                } else {
                    Err(Error::UpperBoundCrossed {
                        bound: HUNDRED_BOUND,
                        value: sum,
                    })
                }
            })
    }

    pub fn checked_sub(self, other: Self) -> FinanceResult<Self> {
        self.0
            .checked_sub(other.0)
            .map(Self::from_permille)
            .ok_or(Error::overflow_err("while subtracting", self, other))
    }
}

// Method used for deserialization
impl<const UPPER_BOUND: Units> TryFrom<Units> for BoundPercent<UPPER_BOUND> {
    type Error = Error;

    fn try_from(permille: Units) -> Result<Self, Self::Error> {
        (permille <= UPPER_BOUND)
            .then(|| Self::new_internal(permille))
            .ok_or(Error::UpperBoundCrossed {
                bound: UPPER_BOUND,
                value: permille,
            })
    }
}

// Method used for serialization
impl<const UPPER_BOUND: Units> From<BoundPercent<UPPER_BOUND>> for Units {
    fn from(percent: BoundPercent<UPPER_BOUND>) -> Self {
        percent.0
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

impl<const UPPER_BOUND: Units> Div for BoundPercent<UPPER_BOUND> {
    type Output = Units;

    fn div(self, rhs: Self) -> Self::Output {
        debug_assert!(!rhs.is_zero());

        self.0.div(rhs.0)
    }
}

impl<const UPPER_BOUND: Units> Mul<BoundPercent<UPPER_BOUND>> for BoundPercent<UPPER_BOUND> {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        Self::from_permille(self.0.mul(rhs.0))
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

impl<const UPPER_BOUND: Units> TryFrom<Amount> for BoundPercent<UPPER_BOUND> {
    type Error = <Units as TryFrom<Amount>>::Error;

    fn try_from(value: Amount) -> Result<Self, Self::Error> {
        Ok(Self::from_permille(value.try_into()?))
    }
}

impl<const UPPER_BOUND: Units> TryFrom<BoundPercent<UPPER_BOUND>> for u16 {
    type Error = TryFromIntError;

    fn try_from(percent: BoundPercent<UPPER_BOUND>) -> Result<Self, Self::Error> {
        percent.0.try_into()
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
