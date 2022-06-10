use std::{
    fmt::Debug,
    ops::{Div, Mul},
};

use cosmwasm_std::Fraction;

use crate::duration::{Duration, Units};

use super::{Fractionable, Integer};

impl<U, D> Fractionable<U> for Duration
where
    Units: Integer<DoubleInteger = D> + TryFrom<D>,
    D: From<Units> + From<U> + Mul<D, Output = D> + Div<D, Output = D>,
    <Units as TryFrom<D>>::Error: Debug,
    U: PartialEq,
{
    fn safe_mul<F>(self, fraction: &F) -> Self
    where
        F: Fraction<U>,
    {
        let nanos = self.nanos().safe_mul(fraction);
        Self::from_nanos(nanos)
    }
}
