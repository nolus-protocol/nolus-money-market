use std::{
    fmt::Debug,
    ops::{Div, Mul},
};

use crate::{duration::{Duration, Units}, ratio::Ratio};

use super::{Fractionable, Integer};

impl<U, D> Fractionable<U> for Duration
where
    Units: Integer<DoubleInteger = D> + TryFrom<D>,
    D: From<Units> + From<U> + Mul<D, Output = D> + Div<D, Output = D>,
    <Units as TryFrom<D>>::Error: Debug,
    U: PartialEq,
{
    fn safe_mul<R>(self, ratio: &R) -> Self
    where
        R: Ratio<U>,
    {
        let nanos = self.nanos().safe_mul(ratio);
        Self::from_nanos(nanos)
    }
}
