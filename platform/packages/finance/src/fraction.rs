use std::fmt::Debug;

use crate::{fractionable::Fragmentable, zero::Zero};

/// A fraction is <= 1 (100%) that applied to a `whole` returns a part of it.
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
