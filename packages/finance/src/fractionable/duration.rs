use cosmwasm_std::{Uint128, Uint256};

use crate::{coin::Coin, currency::Currency, duration::Duration, ratio::Ratio};

use super::{Fractionable, HigherRank};

impl<T> HigherRank<T> for u128
where
    T: Into<Self>,
{
    type Type = Uint256;
    type Intermediate = Uint128;
}

impl<C> Fractionable<Coin<C>> for Duration
where
    C: Currency + PartialEq + Default + Copy,
{
    fn safe_mul<F>(self, fraction: &F) -> Self
    where
        F: Ratio<Coin<C>>,
    {
        let d128: u128 = self.into();
        // TODO re-assess the design of Ratio ... and whether it could be > 1
        d128.safe_mul(fraction)
            .try_into()
            .expect("overflow computing a fraction of duration")
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        coin::Coin, duration::Duration, fractionable::Fractionable, ratio::Rational,
        test::currency::Nls,
    };

    #[test]
    fn safe_mul() {
        let d = Duration::from_secs(10);
        let res = d.safe_mul(&Rational::new(Coin::<Nls>::new(10), Coin::<Nls>::new(20)));
        assert_eq!(Duration::from_secs(5), res);
    }

    #[test]
    fn safe_mul_max() {
        let d = Duration::from_secs(10);
        let res = d.safe_mul(&Rational::new(
            Coin::<Nls>::new(u128::MAX),
            Coin::<Nls>::new(u128::MAX / 2),
        ));
        assert_eq!(Duration::from_secs(20), res);
    }
}
