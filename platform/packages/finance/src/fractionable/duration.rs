use bnum::types::U256;

use crate::{coin::Coin, duration::Duration, ratio::RatioLegacy};

use super::{FractionableLegacy, HigherRank};

impl<T> HigherRank<T> for u128
where
    T: Into<Self>,
{
    type Type = U256;
}

impl<C> FractionableLegacy<Coin<C>> for Duration {
    #[track_caller]
    fn safe_mul<F>(self, fraction: &F) -> Self
    where
        F: RatioLegacy<Coin<C>>,
    {
        let d128: u128 = self.into();
        // TODO re-assess the design of Ratio ... and whether it could be > 1
        d128.safe_mul(fraction).try_into().expect(
            "TODO remove when refactor Fractionable. Overflow computing a fraction of duration",
        )
    }
}

#[cfg(test)]
mod tests {

    use crate::{
        duration::Duration, fractionable::FractionableLegacy, ratio::SimpleFraction, test::coin,
    };

    #[test]
    fn safe_mul() {
        let d = Duration::from_secs(10);
        let res = d.safe_mul(&SimpleFraction::new(coin::coin1(10), coin::coin1(20)));
        assert_eq!(Duration::from_secs(5), res);
    }

    #[test]
    fn safe_mul_max() {
        let d = Duration::from_secs(10);
        let res = d.safe_mul(&SimpleFraction::new(
            coin::coin1(u128::MAX),
            coin::coin1(u128::MAX / 2),
        ));

        assert_eq!(Duration::from_secs(20), res);
    }
}
