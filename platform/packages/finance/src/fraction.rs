use std::fmt::Debug;

use crate::{
    fractionable::{Fractionable, FractionableLegacy, IntoMax},
    zero::Zero,
};

/// A part of a whole
///
/// Never greater than the whole
pub trait Fraction<U> {
    fn of<A>(&self, whole: A) -> A
    where
        U: IntoMax<A::CommonDouble>,
        A: Fractionable<U>;
}

// TODO remove when all Fractionable usages are replaced
pub trait FractionLegacy<U> {
    fn of<A>(&self, whole: A) -> A
    where
        A: FractionableLegacy<U>;
}

pub trait Unit
where
    Self: Copy + Debug + PartialOrd + Zero,
{
}
