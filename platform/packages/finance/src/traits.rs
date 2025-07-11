use std::fmt::Debug;

use crate::zero::Zero;

pub trait FractionUnit
where
    Self: Copy + Debug + Ord + Zero,
{
}
