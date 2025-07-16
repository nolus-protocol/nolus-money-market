use std::fmt::Debug;

use serde::{Deserialize, Serialize};

use crate::{
    arithmetic::{Bits, FractionUnit, One, Trim},
    fraction::Fraction,
    fractionable::Fractionable,
    zero::Zero,
};

// TODO review whether it may gets simpler if extend Fraction
pub trait Ratio<U> {
    fn parts(&self) -> U;
    fn total(&self) -> U;
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Rational<U> {
    nominator: U,
    denominator: U,
}

impl<U> Rational<U>
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

    pub fn nominator(&self) -> U {
        self.nominator
    }

    pub fn denominator(&self) -> U {
        self.denominator
    }

}

impl<U> Bits for Rational<U>
where
    U: Bits,
{
    const BITS: u32 = U::BITS;

    fn leading_zeros(self) -> u32 {
        self.nominator
            .leading_zeros()
            .min(self.denominator.leading_zeros())
    }
}

impl<U> CheckedMul for Rational<U>
where
    U: CheckedMul<U, Output = U> + FractionUnit,
{
    type Output = Self;

    fn checked_mul(self, rhs: Self) -> Option<Self::Output> {
        todo!("Implement")
    }
}

impl<U> Div for Rational<U>
where
    U: CheckedMul<U, Output = U> + FractionUnit,
{
    type Output = Self;

    // (a / b) ÷ (c / d) = (a * d) / (b * c)
    fn div(self, rhs: Self) -> Self::Output {
        todo!("Implement")
    }
}

impl<U, T> Fraction<U> for Rational<T>
where
    Self: Ratio<U>,
{
    #[track_caller]
    fn of<A>(&self, whole: A) -> A
    where
        A: Fractionable<U>,
    {
        todo!("To remove")
    }
}

impl<U> One for Rational<U>
where
    U: FractionUnit + One,
{
    const ONE: Self = Self {
        nominator: U::ONE,
        denominator: U::ONE,
    };
}

impl<U, T> Ratio<U> for Rational<T>
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

impl<U> Trim for Rational<U>
where
    U: FractionUnit,
{
    fn trim(self, bits: u32) -> Self {
        Self::new(self.nominator.trim(bits), self.denominator.trim(bits))
    }
}
