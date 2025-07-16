use crate::{arithmetic::FractionUnit, fractionable::Fractionable};

pub trait Fraction<U>
where
    U: FractionUnit,
{
    fn of<A>(&self, whole: A) -> A
    where
        A: Fractionable<U>,
        U: Into<A::MaxRank>;
}
