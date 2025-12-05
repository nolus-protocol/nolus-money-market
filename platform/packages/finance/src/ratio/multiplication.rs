use std::ops::Div;

use crate::{
    fraction::Unit as FractionUnit,
    fractionable::{Fractionable, IntoMax, TryFromMax, checked_mul::CheckedMul},
    ratio::SimpleFraction,
};

impl<U> SimpleFraction<U>
where
    U: FractionUnit,
{
    pub fn checked_mul<M>(&self, rhs: M) -> Option<M>
    where
        U: IntoMax<M::CommonDouble>,
        M: Fractionable<U>,
    {
        if self.nominator == self.denominator {
            Some(rhs)
        } else {
            let nominator_max = self.nominator.into_max();
            let rhs_max = rhs.into_max();
            let denominator_max = self.denominator.into_max();

            nominator_max
                .checked_mul(rhs_max)
                .map(|product| product.div(denominator_max))
                .and_then(TryFromMax::try_from_max)
        }
    }
}
