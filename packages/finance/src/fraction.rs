use crate::fractionable::Fractionable;

trait Fraction<U> {

    fn of<A>(&self, whole: A) -> A
    where
        A: Fractionable<U>;
}
