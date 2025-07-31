use std::{fmt::Debug, ops::Div};

use serde::{Deserialize, Serialize};

use crate::{
    arithmetics::CheckedMul,
    fractionable::{Fractionable, Fragmentable},
    rational::Rational,
    traits::FractionUnit,
    zero::Zero,
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
}

impl<U> SimpleFraction<U>
where
    U: FractionUnit,
{
    pub fn lossy_mul<F>(self, rhs: F) -> Option<F>
    where
        F: Fractionable<U>,
    {
        if self.nominator == self.denominator {
            Some(rhs)
        } else {
            F::HigherPrimitive::from(self.nominator)
                .checked_mul(F::HigherPrimitive::from(rhs))
                .and_then(|nominator| {
                    let result = nominator.div(self.denominator.into());
                    result.try_into().ok()
                })
        }
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
    fn of<A>(self, whole: A) -> Option<A>
    where
        A: Fragmentable<U>,
    {
        Some(whole.safe_mul(&self))
    }
}
