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
    type Times: Copy + Debug + PartialEq + Zero;

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
    U: Copy + Debug + PartialEq<U> + Scalar + Zero,
{
    #[track_caller]
    pub fn new(nominator: U, denominator: U) -> Self {
        debug_assert_ne!(denominator, Zero::ZERO);

        let (nominator, denominator) = into_coprime(nominator, denominator);

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

fn into_coprime<T>(a: T, b: T) -> (T, T)
where
    T: Copy + Debug + PartialEq + Scalar + Zero,
{
    debug_assert_ne!(b, Zero::ZERO, "RHS-value is zero!");

    let gcd = a.gcd(b);

    debug_assert_ne!(gcd, Zero::ZERO);
    debug_assert!(
        a.modulo(gcd) == Zero::ZERO,
        "LHS-value is not divisible by the GCD!"
    );
    debug_assert!(
        b.modulo(gcd) == Zero::ZERO,
        "RHS-value is not divisible by the GCD!"
    );

    (a.scale_down(gcd), b.scale_down(gcd))
}
