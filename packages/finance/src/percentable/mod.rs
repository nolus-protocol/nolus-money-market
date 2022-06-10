mod coin;
mod duration;
mod percent;
mod u128;
mod u64;

use std::{
    fmt::Debug,
    ops::{Div, Mul},
};

use cosmwasm_std::Fraction;

use crate::{duration::Units as TimeUnits, percent::Units as PercentUnits};
pub trait Fractionable<U> {
    fn safe_mul<F>(self, fraction: &F) -> Self
    where
        F: Fraction<U>;
}

pub trait Percentable: Fractionable<PercentUnits> {}
pub trait TimeSliceable: Fractionable<TimeUnits> {}

pub trait Integer {
    type DoubleInteger;
}

impl<T, D, U> Fractionable<U> for T
where
    T: Integer<DoubleInteger = D> + TryFrom<D>,
    D: From<T> + From<U> + Mul<D, Output = D> + Div<D, Output = D>,
    <T as TryFrom<D>>::Error: Debug,
    U: PartialEq,
{
    fn safe_mul<F>(self, fraction: &F) -> Self
    where
        F: Fraction<U>,
    {
        // TODO debug_assert_eq!(T::BITS * 2, D::BITS);
        if fraction.numerator() == fraction.denominator() {
            self
        } else {
            let res_double: D = D::from(self) * D::from(fraction.numerator());
            let res_double = res_double / D::from(fraction.denominator());
            res_double.try_into().expect("unexpected overflow")
        }
    }
}

impl<T> Percentable for T where T: Fractionable<PercentUnits> {}
impl<T> TimeSliceable for T where T: Fractionable<TimeUnits> {}
