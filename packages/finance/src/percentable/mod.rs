mod coin;
mod percent;
mod u128;
mod u64;

use std::{
    fmt::Debug,
    ops::{Div, Mul},
};

use cosmwasm_std::Fraction;

use crate::percent::Units;

pub trait Percentable {

    fn safe_mul<F>(self, fraction: &F) -> Self
    where
        F: Fraction<Units>;
}

pub trait Integer {
    type DoubleInteger;
}

impl<T, D> Percentable for T
where
    T: Integer<DoubleInteger = D> + TryFrom<D>,
    D: From<T> + From<Units> + Mul<D, Output = D> + Div<D, Output = D>,
    <T as TryFrom<D>>::Error: Debug,
{
    fn safe_mul<F>(self, fraction: &F) -> Self
    where
        F: Fraction<Units>,
    {
        // TODO debug_assert_eq!(T::BITS * 2, D::BITS);
        let res_double: D = D::from(self) * D::from(fraction.numerator());
        let res_double = res_double / D::from(fraction.denominator());
        res_double.try_into().expect("unexpected overflow")
    }
}
