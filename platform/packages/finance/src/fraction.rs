use std::fmt::Debug;

use crate::{
    fractionable::{Fractionable, MaxDoublePrimitive, ToDoublePrimitive},
    zero::Zero,
};

/// A part of a whole
///
/// Never greater than the whole
pub trait Fraction<U> {
    fn of<A>(&self, whole: A) -> A
    where
        U: ToDoublePrimitive,
        A: MaxDoublePrimitive<U>;
}

// TODO remove when all Fractionable usages are replaced
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
