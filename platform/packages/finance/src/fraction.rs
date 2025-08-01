use std::fmt::Debug;

use crate::{fractionable::Fragmentable, zero::Zero};

pub trait Fraction<U> {
    fn of<A>(&self, whole: A) -> A
    where
        A: Fragmentable<U>;
}

pub trait Unit
where
    Self: Copy + Debug + Ord + Zero,
{
}
