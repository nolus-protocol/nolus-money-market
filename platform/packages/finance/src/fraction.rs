use std::fmt::Debug;

use crate::{fractionable::Fractionable, zero::Zero};

/// A part of a whole
///
/// Never greater than the whole
pub trait FractionLegacy<U> {
    fn of<A>(&self, whole: A) -> A
    where
        A: Fractionable<U>;
}

pub trait Unit
where
    Self: Copy + Debug + PartialOrd + Zero,
{
}
