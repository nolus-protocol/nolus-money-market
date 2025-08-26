use std::{
    fmt::Debug,
    ops::{Div, Mul},
};

use crate::{
    duration::Units as TimeUnits, percent::Units as PercentUnits, ratio::Ratio, zero::Zero,
};

mod coin;
mod duration;
mod percent;
mod price;
mod usize;

pub trait Fractionable<U> {
    #[track_caller]
    fn safe_mul<F>(self, fraction: &F) -> Self
    where
        F: Ratio<U>;
}

// TODO revisit its usability
pub trait Percentable: Fractionable<PercentUnits> {}
pub trait TimeSliceable: Fractionable<TimeUnits> {}

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
        R: Ratio<U>,
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

impl<T> Percentable for T where T: Fractionable<PercentUnits> {}
impl<T> TimeSliceable for T where T: Fractionable<TimeUnits> {}
