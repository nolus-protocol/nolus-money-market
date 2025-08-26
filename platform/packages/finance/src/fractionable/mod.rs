use std::{
    fmt::Debug,
    ops::{Div, Mul},
};

use crate::{arithmetics::CheckedMul, ratio::Ratio, zero::Zero};

mod coin;
mod duration;
mod percent;
mod price;
mod usize;

#[allow(dead_code)]
pub(crate) trait Fractionable<U>
where
    Self: Sized + ToPrimitive<Self::HigherPrimitive> + TryFromPrimitive<Self::HigherPrimitive>,
    U: ToPrimitive<Self::HigherPrimitive>,
{
    type HigherPrimitive: CheckedMul<Output = Self::HigherPrimitive>
        + Div<Output = Self::HigherPrimitive>;
}

pub(crate) trait ToPrimitive<P> {
    fn into_primitive(self) -> P;
}

pub(crate) trait TryFromPrimitive<P>
where
    Self: Sized,
{
    #[allow(dead_code)]
    fn try_from_primitive(primitive: P) -> Option<Self>;
}

pub trait Fragmentable<U> {
    #[track_caller]
    fn safe_mul<F>(self, fraction: &F) -> Self
    where
        F: Ratio<U>;
}

// TODO: Remove when removing Fragmentable
pub trait HigherRank<T> {
    type Type;
    // An intermediate type to handle cases when there is no TryInto<Self> for HigherRank::Type but
    // instead there is TryInto<HigherRank::Intermediate> for HigherRank::Type, and Into<Self> for HigherRank::Intermediate
    type Intermediate;
}

impl<T, D, DIntermediate, U> Fragmentable<U> for T
where
    T: HigherRank<U, Type = D, Intermediate = DIntermediate> + Into<D>,
    D: TryInto<DIntermediate>,
    <D as TryInto<DIntermediate>>::Error: Debug,
    DIntermediate: Into<T>,
    D: Mul<D, Output = D> + Div<D, Output = D>,
    U: Zero + PartialEq + Into<D>,
{
    #[track_caller]
    fn safe_mul<R>(self, ratio: &R) -> Self
    where
        R: Ratio<U>,
    {
        // TODO debug_assert_eq!(T::BITS * 2, D::BITS);

        if ratio.parts() == ratio.total() {
            self
        } else {
            let res_double: D = self.into() * ratio.parts().into();
            let res_double = res_double / ratio.total().into();
            let res_intermediate: DIntermediate =
                res_double.try_into().expect("unexpected overflow");
            res_intermediate.into()
        }
    }
}
