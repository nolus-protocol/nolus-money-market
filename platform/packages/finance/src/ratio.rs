use std::fmt::Debug;

use serde::{Deserialize, Serialize};

use crate::{fraction::Fraction, fractionable::Fractionable, zero::Zero};

// TODO review whether it may gets simpler if extend Fraction
pub trait RatioLegacy<U> {
    fn parts(&self) -> U;
    fn total(&self) -> U;
}

/// Defines scalar operations on a value
pub trait Scalar
where
    Self: Sized,
{
    type Times: Debug + Zero;

    fn gcd(self, other: Self) -> Self::Times;

    /// Multiplies `self` by the given `scale`
    fn scale_up(self, scale: Self::Times) -> Option<Self>;

    /// Divides `self` by the given `scale`
    fn scale_down(self, scale: Self::Times) -> Self;

    /// Returns the remainder of `self` divided by `scale`
    fn modulo(self, scale: Self::Times) -> Self::Times;

    fn into_times(self) -> Self::Times;
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[cfg_attr(any(test, feature = "testing"), derive(PartialEq,))]
#[serde(rename_all = "snake_case")]
pub struct Rational<U> {
    nominator: U,
    denominator: U,
}

impl<U> Rational<U>
where
    U: Debug + PartialEq<U> + Scalar + Zero,
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

impl<U, T> Fraction<U> for Rational<T>
where
    Self: RatioLegacy<U>,
{
    #[track_caller]
    fn of<A>(&self, whole: A) -> A
    where
        A: Fractionable<U>,
    {
        whole.safe_mul(self)
    }
}

impl<U, T> RatioLegacy<U> for Rational<T>
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
