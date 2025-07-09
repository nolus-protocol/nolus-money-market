use crate::{fractionable::Fractionable, traits::FractionUnit};

pub trait Rational<U>
where
    U: FractionUnit,
{
    /// Computes the fraction of a given whole.
    fn of<A>(self, whole: A) -> Option<A>
    where
        U: Into<A::MaxRank>,
        A: Fractionable<U>;
}
