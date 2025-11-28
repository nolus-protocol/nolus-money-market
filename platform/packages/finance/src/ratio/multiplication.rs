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

    fn cross_normalize(&self, rhs: Self) -> (Self, Self) {
        // from (a / b) and (c / d), to (a / d) and (c / b)
        (
            Self::new(self.nominator, rhs.denominator),
            Self::new(rhs.nominator, self.denominator),
        )
    }
}

/// Checked multiplication of two `SimpleFraction`-s
/// Returns `None` if either the numerator or denominator multiplication overflows
impl<U> CheckedMul for SimpleFraction<U>
where
    U: CheckedMul<U, Output = U> + FractionUnit,
{
    type Output = Self;

    fn checked_mul(self, rhs: Self) -> Option<Self::Output> {
        // (a / b).checked_mul(c / d) = (a / d).checked_mul(c / b)
        // => (a1.checked_mul(c1)) / (d1.checked_mul(b1))
        // where a1, b1, c1 and d1 are normalized
        let (ad, cb) = self.cross_normalize(rhs);

        ad.nominator
            .checked_mul(cb.nominator)
            .and_then(|nominator| {
                ad.denominator
                    .checked_mul(cb.denominator)
                    .map(|denominator| Self::new(nominator, denominator))
            })
    }
}

#[cfg(test)]
mod test {
    use bnum::types::U256;

    use crate::{
        coin::Amount, fractionable::checked_mul::CheckedMul, percent::Units as PercentUnits,
        ratio::SimpleFraction,
    };

    #[test]
    fn cross_normalize() {
        let a = fraction(12, 25);
        let b = fraction(35, 9);

        assert_eq!((fraction(4, 3), fraction(7, 5)), a.cross_normalize(b))
    }

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

    fn fraction(nom: Amount, denom: Amount) -> SimpleFraction<Amount> {
        SimpleFraction::new(nom, denom)
    }

    fn u_256(quantity: PercentUnits) -> U256 {
        U256::from(quantity)
    }
}
