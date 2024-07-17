use crate::{error::Result as FinanceResult, fractionable::Fractionable};

pub trait Fraction<U> {
    fn of<A>(&self, whole: A) -> FinanceResult<A>
    where
        A: Fractionable<U>;
}
