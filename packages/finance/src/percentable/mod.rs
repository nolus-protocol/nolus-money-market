mod coin;
mod u64;
mod u128;
mod percent;

use std::ops::{Div, Mul};

use crate::percent::Percent;

pub trait Percentable: Mul<Percent, Output = <Self as Percentable>::Intermediate> {
    type Intermediate: Div<Percent, Output = <Self as Percentable>::Result>;
    type Result: Percentable;
}