use std::fmt::Debug;

use crate::{fractionable::FractionableLegacy, zero::Zero};

/// A part of a whole
///
/// Never greater than the whole
pub trait Fraction<U> {
    fn of<A>(&self, whole: A) -> A
    where
        A: FractionableLegacy<U>;
}
pub trait Unit
where
    Self: Copy + Debug + PartialOrd + Sized + Zero,
{
    type Times: Copy + Debug + PartialEq + Zero;

    fn gcd<U>(self, other: U) -> Self::Times
    where
        U: Unit<Times = Self::Times>;

    /// Divides `self` by the given `scale`
    ///
    /// [scale] should be non zero
    fn scale_down(self, scale: Self::Times) -> Self;

    /// Returns the remainder of `self` divided by `scale`
    ///
    /// [scale] should be non zero
    fn modulo(self, scale: Self::Times) -> Self::Times;

    fn primitive(self) -> Self::Times;
}

pub trait Coprime
where
    Self: Unit,
{
    /// [other] should be nonzero
    fn coprime_with<U>(self, other: U) -> (Self, U)
    where
        U: Unit<Times = Self::Times>;
}

impl<T> Coprime for T
where
    T: Unit,
{
    fn coprime_with<U>(self, other: U) -> (Self, U)
    where
        U: Unit<Times = Self::Times>,
    {
        debug_assert_ne!(other, Zero::ZERO, "RHS-value is zero!");

        let gcd = self.gcd(other);

        debug_assert_ne!(gcd, Zero::ZERO);
        debug_assert!(
            self.modulo(gcd) == Zero::ZERO,
            "LHS-value is not divisible by the GCD!"
        );
        debug_assert!(
            other.modulo(gcd) == Zero::ZERO,
            "RHS-value is not divisible by the GCD!"
        );

        (self.scale_down(gcd), other.scale_down(gcd))
    }
}
