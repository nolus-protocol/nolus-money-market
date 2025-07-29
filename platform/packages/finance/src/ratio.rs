use std::{fmt::Debug, ops::Mul};

use serde::{Deserialize, Serialize};

use crate::{fractionable::Fractionable, rational::Rational, traits::FractionUnit, zero::Zero};

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

    // TODO remove it when implement Ord for SimpleFraction
    pub fn min(self, other: SimpleFraction<U>) -> SimpleFraction<U>
    where
        U: Mul<Output = U>,
    {
        // a / b, c /d compare them by cross-multiplication
        // if ad = bc => a / b = c / d
        // if ad < bc => a / b < c / d
        // if ad > bc => a / b > c / d

        let ad = self.nominator.mul(other.denominator);
        let bc = self.denominator.mul(other.denominator);
        if ad <= bc { self } else { other }
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
        A: Fractionable<U>,
    {
        Some(whole.safe_mul(&self))
    }
}
