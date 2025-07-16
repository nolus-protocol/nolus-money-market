use std::fmt::Debug;

use crate::zero::Zero;

pub trait Bits {
    const BITS: u32;

    fn leading_zeros(self) -> u32;
}

pub trait CheckedAdd<Rhs = Self> {
    type Output;

    fn checked_add(self, rhs: Rhs) -> Option<Self::Output>;
}

pub trait CheckedMul<Rhs = Self> {
    type Output;

    fn checked_mul(self, rhs: Rhs) -> Option<Self::Output>;
}

pub trait FractionUnit
where
    Self: Copy + Debug + Ord + Scalar + Trim + Zero,
{
}

pub trait One {
    const ONE: Self;
}

pub trait Scalar
where
    Self: Copy + Sized,
{
    type Base: Copy + Debug + PartialEq + Zero;

    fn gcd(self, other: Self) -> Self::Base;

    /// Multiplies `self` by the given `scale`
    fn scale_up(self, scale: Self::Base) -> Option<Self>;

    /// Divides `self` by the given `scale`
    fn scale_down(self, scale: Self::Base) -> Self;

    /// Returns the remainder of `self` divided by `scale`
    fn modulo(self, scale: Self::Base) -> Self::Base;

    fn into_base(self) -> Self::Base;
}

pub trait Trim
where
    Self: Bits + Copy,
{
    /// Trims off the highest bits by shifting right
    fn trim(self, bits: u32) -> Self;
}

pub fn into_coprime<T>(a: T, b: T) -> (T, T)
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
