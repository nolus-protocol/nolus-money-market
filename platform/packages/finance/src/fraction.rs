use crate::fractionable::Fractionable;

pub trait Fraction<U> {
    // Parts mut not exceed total
    fn parts(&self) -> U;
    fn total(&self) -> U;

    /// Computes the fraction of a given whole.
    fn of<A>(&self, whole: A) -> A
    where
        A: Fractionable<U>;
}
