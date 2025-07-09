use std::ops::Div;

use crate::traits::{CheckedMul, One, Trim};

mod coin;
mod duration;
mod percent;
mod price;
mod usize;

pub trait Fractionable<U>
where
    Self: Copy,
{
    /// The larger numeric type between U and Self, used to safely perform intermediate calculations.
    type MaxRank: CheckedMul<Output = Self::MaxRank>
        + Div<Output = Self::MaxRank>
        + From<Self>
        + From<U>
        + One
        + Trim
        + TryInto<Self>;
}
