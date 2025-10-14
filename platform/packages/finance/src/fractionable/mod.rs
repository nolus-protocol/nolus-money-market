use std::{
    fmt::Debug,
    ops::{Div, Mul},
};

use crate::{fractionable::checked_mul::CheckedMul, ratio::RatioLegacy, zero::Zero};

pub(crate) mod checked_mul;
mod usize;

/// Converts the domain type into its wider primitive `Double` type
pub trait ToDoublePrimitive {
    type Double;

    fn to_double(&self) -> Self::Double;
}

/// Defines a common `Max` type, chosen as one of `Double` the types from either `Self` or `Other`
pub trait CommonDoublePrimitive<Other> {
    type CommonDouble: CheckedMul<Output = Self::CommonDouble> + Div<Output = Self::CommonDouble>;
}

/// Domain entity for which a fraction could be calculated.
pub trait Fractionable<FractionUnit>
where
    Self: CommonDoublePrimitive<FractionUnit>
        + TryFromMax<<Self as CommonDoublePrimitive<FractionUnit>>::CommonDouble>
        + Sized,
{
}

pub trait IntoMax<Max>
where
    Self: ToDoublePrimitive,
{
    fn into_max(self) -> Max;
}

/// Conversion from `Max` back to the domain type
pub trait TryFromMax<Max>
where
    Self: IntoMax<Max> + Sized,
{
    fn try_from_max(max: Max) -> Option<Self>;
}

pub trait FractionableLegacy<U> {
    #[track_caller]
    fn safe_mul<F>(self, fraction: &F) -> Self
    where
        F: RatioLegacy<U>;
}

pub trait HigherRank<T> {
    type Type;
}

impl<T, D, U> FractionableLegacy<U> for T
where
    T: HigherRank<U, Type = D> + Into<D>,
    D: Mul<D, Output = D> + Div<D, Output = D> + TryInto<T>,
    <D as TryInto<T>>::Error: Debug,
    U: Zero + PartialEq + Into<D>,
{
    #[track_caller]
    fn safe_mul<R>(self, ratio: &R) -> Self
    where
        R: RatioLegacy<U>,
    {
        // TODO debug_assert_eq!(T::BITS * 2, D::BITS);

        if ratio.parts() == ratio.total() {
            self
        } else {
            let res_mul: D = self.into() * ratio.parts().into();
            let res_div = res_mul / ratio.total().into();
            res_div.try_into().expect("unexpected overflow")
        }
    }
}
