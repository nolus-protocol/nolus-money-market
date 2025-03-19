use sdk::{
    cosmwasm_std::Uint256,
    schemars::{self, JsonSchema},
};
use serde::{Deserialize, Serialize};
use std::{
    fmt::{Display, Formatter, Result as FmtResult, Write},
    marker::PhantomData,
    ops::{Div, Rem},
};

use crate::{
    error::{Error, Result as FinanceResult},
    fractionable::Fractionable,
    ratio::{CheckedAdd, CheckedMul, Ratio, Rational},
    zero::Zero,
};

use super::{Percent100, Units};

#[derive(
    Clone, Copy, Debug, Default, Deserialize, Eq, JsonSchema, Ord, PartialEq, PartialOrd, Serialize,
)]
#[serde(try_from = "Units")]
pub struct BoundPercent<B>
where
    B: UpperBound,
{
    units: Units,
    #[serde(skip)]
    _bound: PhantomData<B>,
}

impl<B> BoundPercent<B>
where
    B: UpperBound,
{
    pub const ZERO: Self = Self::from_permille(0);
    pub const HUNDRED: Self = Self::from_permille(Self::PERMILLE);
    pub(crate) const PERMILLE: Units = 1000;

    const UNITS_TO_PERCENT_RATIO: Units = 10;

    #[cfg(any(test, feature = "testing"))]
    pub fn new(units: Units) -> Self {
        Self::new_internal(units)
    }

    const fn new_internal(units: Units) -> Self {
        debug_assert!(units <= B::BOUND, "Value exceeds upper bound!");
        Self {
            units,
            _bound: PhantomData,
        }
    }

    pub fn from_percent(percent: u16) -> Self {
        Self::from_permille(Units::from(percent) * Self::UNITS_TO_PERCENT_RATIO)
    }

    pub const fn from_permille(permille: Units) -> Self {
        Self::new_internal(permille)
    }

    pub const fn units(&self) -> Units {
        self.units
    }

    pub fn is_zero(&self) -> bool
    where
        B: PartialEq,
    {
        self == &Self::ZERO
    }

    pub fn checked_add(self, other: Self) -> FinanceResult<Self> {
        self.units
            .checked_add(other.units)
            .map(Self::from_permille)
            .ok_or(Error::overflow_err("while adding", self, other))
    }

    pub fn checked_sub(self, other: Self) -> FinanceResult<Self> {
        self.units
            .checked_sub(other.units)
            .map(Self::from_permille)
            .ok_or(Error::overflow_err("while subtracting", self, other))
    }
}

impl BoundPercent<HundredBound> {
    pub fn of<A>(&self, whole: A) -> A
    where
        A: Fractionable<Units>,
    {
        let ratio: Ratio<Units> = self.into();
        whole.safe_mul(&ratio)
    }
}

impl BoundPercent<MaxBound> {
    pub fn of<A>(&self, whole: A) -> Option<A>
    where
        Units: CheckedMul<A, Output = A>,
        A: CheckedAdd<Output = A> + Copy + Fractionable<Units>,
    {
        let ratio: Rational<Units> = self.into();
        ratio.checked_mul(whole)
    }
}

impl From<&BoundPercent<HundredBound>> for Ratio<Units> {
    fn from(percent: &BoundPercent<HundredBound>) -> Self {
        Self::new(percent.units, Percent100::HUNDRED.units)
    }
}

impl From<&BoundPercent<MaxBound>> for Rational<Units> {
    fn from(percent: &BoundPercent<MaxBound>) -> Self {
        Self::new(percent.units, Percent100::HUNDRED.units)
    }
}

impl<U> From<Ratio<U>> for BoundPercent<HundredBound>
where
    U: Copy,
    Self: Fractionable<U>,
{
    fn from(ratio: Ratio<U>) -> Self {
        Self::HUNDRED.safe_mul(&ratio)
    }
}

impl<B> TryFrom<Units> for BoundPercent<B>
where
    B: UpperBound,
{
    type Error = &'static str;

    fn try_from(permille: Units) -> Result<Self, Self::Error> {
        (permille <= Self::PERMILLE)
            .then(|| Self::new_internal(permille))
            .ok_or("Percent cannot be greater than 100%")
    }
}

impl<B> From<BoundPercent<B>> for u128
where
    B: UpperBound,
{
    fn from(percent: BoundPercent<B>) -> Self {
        u128::from(percent.units())
    }
}

impl<B> From<BoundPercent<B>> for Uint256
where
    B: UpperBound,
{
    fn from(p: BoundPercent<B>) -> Self {
        Uint256::from(p.units())
    }
}

impl<B> CheckedAdd for BoundPercent<B>
where
    B: UpperBound,
{
    type Output = Self;

    fn checked_add(self, rhs: Self) -> Option<Self::Output> {
        self.checked_add(rhs).ok()
    }
}

impl<B> CheckedMul<BoundPercent<B>> for Units
where
    B: UpperBound,
{
    type Output = BoundPercent<B>;

    fn checked_mul(self, rhs: BoundPercent<B>) -> Option<Self::Output> {
        self.checked_mul(rhs.units())
            .map(BoundPercent::from_permille)
    }
}

impl<B> Zero for BoundPercent<B>
where
    B: UpperBound,
{
    const ZERO: Self = Self::ZERO;
}

impl<B> Div for BoundPercent<B>
where
    B: PartialEq + UpperBound,
{
    type Output = Units;

    fn div(self, rhs: Self) -> Self::Output {
        debug_assert!(!rhs.is_zero());

        self.units / rhs.units
    }
}

impl<B> Rem for BoundPercent<B>
where
    B: PartialEq + UpperBound,
{
    type Output = Self;

    fn rem(self, rhs: Self) -> Self::Output {
        debug_assert!(!rhs.is_zero());
        Self::new_internal(self.units % rhs.units)
    }
}

impl<B> Display for BoundPercent<B>
where
    B: UpperBound,
{
    #[track_caller]
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        let whole = (self.units) / Self::UNITS_TO_PERCENT_RATIO;
        let (no_fraction, overflow) = whole.overflowing_mul(Self::UNITS_TO_PERCENT_RATIO);
        debug_assert!(!overflow);
        let (fractional, overflow) = (self.units).overflowing_sub(no_fraction);
        debug_assert!(!overflow);

        f.write_fmt(format_args!("{}", whole))?;
        if fractional != Units::default() {
            f.write_fmt(format_args!(".{}", fractional))?;
        }
        f.write_char('%')?;
        Ok(())
    }
}

pub trait UpperBound {
    const BOUND: Units;
}

#[derive(Copy, Clone, Debug, Default, Eq, JsonSchema, Ord, PartialEq, PartialOrd)]
pub struct HundredBound;

impl UpperBound for HundredBound {
    const BOUND: Units = 1000;
}

#[derive(Copy, Clone, Eq, JsonSchema, Ord, PartialEq, PartialOrd)]
#[cfg_attr(any(test, feature = "testing"), derive(Debug))]
pub struct MaxBound;

impl UpperBound for MaxBound {
    const BOUND: Units = Units::MAX;
}
