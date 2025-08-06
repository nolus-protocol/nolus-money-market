use crate::fractionable::Fractionable;

pub trait Rational<U> {
    /// Computes the fraction of a given whole.
    fn of<A>(self, whole: A) -> Option<A>
    where
        A: Fractionable<U>;
}
