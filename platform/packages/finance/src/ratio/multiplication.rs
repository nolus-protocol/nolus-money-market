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

// TODO unify the multiplication using the logic from SimpleFraction::checked_mul(Fractionable)
impl<U> CheckedMul for SimpleFraction<U>
where
    U: CheckedMul<U, Output = U> + FractionUnit,
{
    type Output = Self;

    fn checked_mul(self, rhs: Self) -> Option<Self::Output> {
        self.nominator
            .checked_mul(rhs.nominator)
            .and_then(|nominator| {
                self.denominator
                    .checked_mul(rhs.denominator)
                    .map(|denominator| Self::new(nominator, denominator))
            })
    }
}

#[cfg(test)]
mod test {
    use bnum::types::U256;

    use crate::{
        fractionable::checked_mul::CheckedMul, percent::Units as PercentUnits,
        ratio::SimpleFraction,
    };

    #[test]
    fn checked_mul_trait() {
        let lhs = SimpleFraction::new(u_256(350), u_256(1000));
        let rhs = SimpleFraction::new(u_256(400), u_256(1000));
        let exp = SimpleFraction::new(u_256(7), u_256(50));
        assert_eq!(exp, lhs.checked_mul(rhs).unwrap())
    }

    #[test]
    fn checked_mul_trait_overflow() {
        let lhs = SimpleFraction::new(U256::MAX - u_256(1), u_256(1000));
        let rhs = SimpleFraction::new(u_256(3), u_256(1000));
        assert!(lhs.checked_mul(rhs).is_none())
    }

    fn u_256(quantity: PercentUnits) -> U256 {
        U256::from(quantity)
    }
}
