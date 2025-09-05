use std::{
    fmt::Debug,
    ops::{Div, Mul},
};

use crate::{fractionable::checked_mul::CheckedMul, ratio::RatioLegacy, zero::Zero};

pub(crate) mod checked_mul;
mod coin;
mod duration;
mod percent;
mod price;
mod usize;

pub trait ToDoublePrimitive {
    type Double;
}

/// Defines a common `Max` type based on `Self::Double` and `Other::Double` types.
pub trait MaxPrimitive<Other>
where
    Self: ToDoublePrimitive,
    Other: ToDoublePrimitive,
{
    type Max: CheckedMul<Output = Self::Max> + Div<Output = Self::Max>;

    // Having two identical methods so the trait becomes symmetric
    fn self_to_max(self) -> Self::Max;
    fn other_to_max(other: Other) -> Self::Max;
}

pub trait TryFromMaxPrimitive<Max>
where
    Self: Sized,
{
    fn try_from_max(max: Max) -> Option<Self>;
}

pub trait Fractionable<U> {
    #[track_caller]
    fn safe_mul<F>(self, fraction: &F) -> Self
    where
        F: RatioLegacy<U>;
}

pub trait HigherRank<T> {
    type Type;
}

impl<T, D, U> Fractionable<U> for T
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
