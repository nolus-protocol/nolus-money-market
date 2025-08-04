use std::{fmt::Debug, ops::Div};

use serde::{Deserialize, Serialize};

use crate::{
    arithmetics::CheckedMul,
    fraction::Unit as FractionUnit, 
    fractionable::Fragmentable, rational::Rational,
    zero::Zero,
,
};

// TODO review whether it may gets simpler if extend Fraction
pub trait Ratio<U> {
    fn parts(&self) -> U;
    fn total(&self) -> U;
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[cfg_attr(any(test, feature = "testing"), derive(PartialEq,))]
#[serde(rename_all = "snake_case")]
pub struct SimpleFraction<U> {
    nominator: U,
    denominator: U,
}

impl<U> SimpleFraction<U>
where
    U: FractionUnit,
{
    #[track_caller]
    pub fn new(nominator: U, denominator: U) -> Self {
        debug_assert_ne!(denominator, Zero::ZERO);

        Self {
            nominator,
            denominator,
        }
    }

    pub(crate) fn nominator(&self) -> U {
        self.nominator
    }

    pub(crate) fn denominator(&self) -> U {
        self.denominator
    }

    fn inv(self) -> Self {
        Self::new(self.denominator, self.nominator)
    }
}

impl<U> CheckedMul for SimpleFraction<U>
where
    U: CheckedMul<U, Output = U> + FractionUnit,
{
    type Output = Self;

    fn checked_mul(self, rhs: Self) -> Option<Self::Output> {
        self.nominator
            .checked_mul(rhs.nominator)
            .and_then(|nominator| {
                self.denominator
                    .checked_mul(rhs.denominator)
                    .map(|denominator| Self::new(nominator, denominator))
            })
    }
}

impl<U> Div for SimpleFraction<U>
where
    U: CheckedMul<U, Output = U> + FractionUnit,
{
    type Output = Self;

    // (a / b) ÷ (c / d) = (a * d) / (b * c)
    fn div(self, rhs: Self) -> Self::Output {
        debug_assert_ne!(rhs.nominator, Zero::ZERO, "Cannot divide by zero fraction");

        self.checked_mul(rhs.inv())
            .expect("Division should not overflow")
    }
}

impl<U, T> Ratio<U> for SimpleFraction<T>
where
    T: Zero + Copy + PartialEq + Into<U>,
{
    fn parts(&self) -> U {
        self.nominator.into()
    }

    fn total(&self) -> U {
        self.denominator.into()
    }
}

impl<U> Rational<U> for SimpleFraction<U>
where
    U: FractionUnit,
{
    fn of<A>(&self, whole: A) -> Option<A>
    where
        A: Fragmentable<U>,
    {
        Some(whole.safe_mul(self))
    }
}
