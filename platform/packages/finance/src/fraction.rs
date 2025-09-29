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

    fn gcd(self, other: Self) -> Self::Times;

    /// Divides `self` by the given `scale`
    ///
    /// [scale] should be non zero
    fn scale_down(self, scale: Self::Times) -> Self;

    /// Returns the remainder of `self` divided by `scale`
    ///
    /// [scale] should be non zero
    fn modulo(self, scale: Self::Times) -> Self::Times;
}
