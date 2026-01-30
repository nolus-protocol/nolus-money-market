use std::fmt::Debug;

use crate::{
    fractionable::{Fractionable, IntoMax},
    ratio::SimpleFraction,
    zero::Zero,
};

/// A part of a whole
///
/// Never greater than the whole
pub trait Fraction<U> {
    fn of<A>(&self, whole: A) -> A
    where
        U: IntoMax<A::CommonDouble>,
        A: Fractionable<U>;
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

    fn to_primitive(self) -> Self::Times;
}

pub trait Coprime
where
    Self: Unit,
{
    /// [other] should be nonzero
    fn to_coprime_with<U>(self, other: U) -> (Self, U)
    where
        U: Unit<Times = Self::Times>;
}

impl<T> Coprime for T
where
    T: Unit,
{
    fn to_coprime_with<U>(self, other: U) -> (Self, U)
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

/// Implemented by types, which can be represented as [SimpleFraction], allowing for multiplication logic
pub trait ToFraction<U> {
    fn to_fraction(self) -> SimpleFraction<U>;
}
