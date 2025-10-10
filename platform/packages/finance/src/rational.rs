use crate::fractionable::{Fractionable, FractionableLegacy, IntoMax};

/// A rational applied to `whole` returns a part of `whole`.
/// May exceed 1. Returns `None` if computation overflows.
pub trait Rational<U> {
    fn of<A>(&self, whole: A) -> Option<A>
    where
        U: IntoMax<A::CommonDouble>,
        A: Fractionable<U>;
}

// TODO remove when all Fractionable usages are replaced
pub trait RationalLegacy<U> {
    fn of<A>(&self, whole: A) -> Option<A>
    where
        A: FractionableLegacy<U>;
}
