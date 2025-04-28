use std::{
    fmt::{Debug, Display, Formatter, Result as FmtResult, Write},
    ops::{Div, Rem},
};

use serde::{Deserialize, Serialize};

use sdk::cosmwasm_std::Uint256;

use crate::{
    error::{Error, Result as FinanceResult},
    fraction::Fraction,
    fractionable::Fractionable,
    ratio::{CheckedAdd, CheckedMul, Ratio, Rational},
    zero::Zero,
};

use super::{HUNDRED_BOUND, MAX_BOUND, Percent100, Units};

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

    pub fn from_ratio<FractionUnit>(
        nominator: FractionUnit,
        denominator: FractionUnit,
    ) -> Option<Self>
    where
        FractionUnit: Copy + Debug + Div + Ord + PartialEq + Rem<Output = FractionUnit> + Zero,
        <FractionUnit as Div>::Output: CheckedMul<Self, Output = Self>,
        Self: Fractionable<FractionUnit>,
    {
        Rational::new(nominator, denominator).checked_mul(Self::HUNDRED)
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

impl Fraction<Units> for BoundPercent<HUNDRED_BOUND> {
    fn parts(&self) -> Units {
        self.units()
    }

    fn total(&self) -> Units {
        Self::HUNDRED.0
    }

    fn of<A>(&self, whole: A) -> A
    where
        A: Fractionable<Units>,
    {
        debug_assert!(self.parts() <= self.total());

        let ratio: Ratio<Units> = self.into();
        whole.safe_mul(&ratio)
    }
}

impl BoundPercent<MAX_BOUND> {
    pub fn of<A>(&self, whole: A) -> Option<A>
    where
        Units: CheckedMul<A, Output = A>,
        A: CheckedAdd<Output = A> + Copy + Fractionable<Units>,
    {
        let ratio: Rational<Units> = self.into();
        ratio.checked_mul(whole)
    }
}

impl From<&BoundPercent<HUNDRED_BOUND>> for Ratio<Units> {
    fn from(percent: &BoundPercent<HUNDRED_BOUND>) -> Self {
        Self::new(percent.0, Percent100::HUNDRED.0)
    }
}

impl From<&BoundPercent<MAX_BOUND>> for Rational<Units> {
    fn from(percent: &BoundPercent<MAX_BOUND>) -> Self {
        Self::new(percent.0, Percent100::HUNDRED.0)
    }
}

impl From<BoundPercent<HUNDRED_BOUND>> for BoundPercent<MAX_BOUND> {
    fn from(percent: BoundPercent<HUNDRED_BOUND>) -> Self {
        Self::from_permille(percent.0)
    }
}

impl TryFrom<BoundPercent<MAX_BOUND>> for BoundPercent<HUNDRED_BOUND> {
    type Error = Error;

    fn try_from(percent: BoundPercent<MAX_BOUND>) -> Result<Self, Self::Error> {
        (percent.0 <= HUNDRED_BOUND)
            .then(|| Self::from_permille(percent.0))
            .ok_or_else(|| Error::UpperBoundCrossed {
                bound: HUNDRED_BOUND,
                value: percent.0,
            })
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

impl<const UPPER_BOUND: Units> From<BoundPercent<UPPER_BOUND>> for u128 {
    fn from(percent: BoundPercent<UPPER_BOUND>) -> Self {
        u128::from(percent.units())
    }
}

impl<const UPPER_BOUND: Units> From<BoundPercent<UPPER_BOUND>> for Uint256 {
    fn from(p: BoundPercent<UPPER_BOUND>) -> Self {
        Uint256::from(p.units())
    }
}

impl<const UPPER_BOUND: Units> CheckedAdd for BoundPercent<UPPER_BOUND> {
    type Output = Self;

    fn checked_add(self, rhs: Self) -> Option<Self::Output> {
        self.checked_add(rhs).ok()
    }
}

impl<const UPPER_BOUND: Units> CheckedMul<BoundPercent<UPPER_BOUND>> for Units {
    type Output = BoundPercent<UPPER_BOUND>;

    fn checked_mul(self, rhs: BoundPercent<UPPER_BOUND>) -> Option<Self::Output> {
        self.checked_mul(rhs.units())
            .map(BoundPercent::from_permille)
    }
}

impl<const UPPER_BOUND: Units> Zero for BoundPercent<UPPER_BOUND> {
    const ZERO: Self = Self::ZERO;
}

impl<const UPPER_BOUND: Units> Div for BoundPercent<UPPER_BOUND> {
    type Output = Units;

    fn div(self, rhs: Self) -> Self::Output {
        debug_assert!(!rhs.is_zero());

        self.0 / rhs.0
    }
}

impl<const UPPER_BOUND: Units> Rem for BoundPercent<UPPER_BOUND> {
    type Output = Self;

    fn rem(self, rhs: Self) -> Self::Output {
        debug_assert!(!rhs.is_zero());
        Self::new_internal(self.0 % rhs.0)
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

        f.write_fmt(format_args!("{}", whole))?;
        if fractional != Units::default() {
            f.write_fmt(format_args!(".{}", fractional))?;
        }
        f.write_char('%')?;
        Ok(())
    }
}
