use crate::fractionable::{Fractionable, ToPrimitive};

/// `Rational` is the unbounded equivalent of `Fraction<U>`.
pub trait Rational<U> {
    /// Computes the fraction of a given whole.
    fn of<A>(&self, whole: A) -> Option<A>
    where
        A: Fractionable<U>,
        U: ToPrimitive<A::HigherPrimitive>;
}
