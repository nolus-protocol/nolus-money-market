use crate::fractionable::Fractionable;

pub trait Fraction<U> {
    fn of<A>(&self, whole: A) -> A
    where
        A: Fractionable<U>;
}
