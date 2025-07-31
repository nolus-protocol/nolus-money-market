use crate::{fractionable::Fragmentable, traits::FractionUnit};

pub trait Fraction<U>
where
    U: FractionUnit,
{
    fn of<A>(self, whole: A) -> A
    where
        A: Fragmentable<U>;
}
