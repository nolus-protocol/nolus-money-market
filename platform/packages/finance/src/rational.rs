use crate::fractionable::FractionableLegacy;

/// A rational applied to `whole` returns a part of `whole`.
/// May exceed 1. Returns `None` if computation overflows.
pub trait RationalLegacy<U> {
    fn of<A>(&self, whole: A) -> Option<A>
    where
        A: FractionableLegacy<U>;
}
