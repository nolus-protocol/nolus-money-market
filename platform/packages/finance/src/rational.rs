use crate::fractionable::Fractionable;

/// A rational applied to `whole` returns a part of `whole`.
/// May exceed 1. Returns `None` if computation overflows.
pub trait Rational<U> {
    fn of<A>(&self, whole: A) -> Option<A>
    where
        A: Fractionable<U>;
}
