mod coin;
mod deprecated;
mod duration;
mod percent;
mod u128;
mod u64;

use std::{
    fmt::Debug,
    ops::{Div, Mul},
};

use crate::{duration::Units as TimeUnits, percent::Units as PercentUnits, ratio::Ratio};
pub trait Fractionable<U> {
    fn safe_mul<F>(self, fraction: &F) -> Self
    where
        F: Ratio<U>;
}

pub trait Percentable: Fractionable<PercentUnits> {}
pub trait TimeSliceable: Fractionable<TimeUnits> {}

pub trait Integer {
    // An intermediate type to handle cases when there is no TryFrom<DoubleInteger> for Self but
    // instead there is TryFrom<DoubleInteger> for SameBitsInteger, and From<SameBitsInteger> for Self
    type SameBitsInteger;
    type DoubleInteger;
}

impl<T, D, DIntermediate, U> Fractionable<U> for T
where
    T: Integer<SameBitsInteger = DIntermediate, DoubleInteger = D> + From<DIntermediate>,
    D: From<T> + From<U> + Mul<D, Output = D> + Div<D, Output = D>,
    <DIntermediate as TryFrom<D>>::Error: Debug,
    DIntermediate: TryFrom<D>,
    U: PartialEq,
{
    fn safe_mul<R>(self, ratio: &R) -> Self
    where
        R: Ratio<U>,
    {
        // TODO debug_assert_eq!(T::BITS * 2, D::BITS);
        if ratio.parts() == ratio.total() {
            self
        } else {
            let res_double: D = D::from(self) * D::from(ratio.parts());
            let res_double = res_double / D::from(ratio.total());
            let res_intermediate: DIntermediate =
                res_double.try_into().expect("unexpected overflow");
            res_intermediate.into()
        }
    }
}

impl<T> Percentable for T where T: Fractionable<PercentUnits> {}
impl<T> TimeSliceable for T where T: Fractionable<TimeUnits> {}
