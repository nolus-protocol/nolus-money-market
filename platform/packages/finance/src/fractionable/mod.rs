use std::ops::Div;

use crate::arithmetic::{CheckedMul, One, Trim};

mod coin;
mod duration;
mod percent;
mod price;
mod usize;

pub trait Fractionable<U>
where
    Self: Copy,
{
    type MaxRank: CheckedMul<Output = Self::MaxRank>
        + Div<Output = Self::MaxRank>
        + From<U>
        + One
        + Trim
        + TryFrom<Self>
        + TryInto<Self>;
}
