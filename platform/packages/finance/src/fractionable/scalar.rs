use std::fmt::Debug;

use crate::zero::Zero;

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
