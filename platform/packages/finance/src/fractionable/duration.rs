use sdk::cosmwasm_std::{Uint128, Uint256};

use crate::{
    coin::Coin,
    duration::Duration,
    fractionable::{Fractionable, Fragmentable},
    ratio::Ratio,
};
use bnum::types;

use super::HigherRank;

impl<T> HigherRank<T> for u128
where
    T: Into<Self>,
{
    type Type = Uint256;
    type Intermediate = Uint128;
}

impl<C> Fractionable<Coin<C>> for Duration {
    type HigherPrimitive = types::U256;
}

// TODO remove when Fractionable replaces the Fragmentable as trait boundary
impl<C> Fragmentable<Coin<C>> for Duration {
    #[track_caller]
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
    use currency::test::SuperGroupTestC1;

    use crate::{coin::Coin, duration::Duration, ratio::SimpleFraction};

    #[test]
    fn lossy_mul() {
        let d = Duration::from_secs(10);
        let res = SimpleFraction::new(
            Coin::<SuperGroupTestC1>::new(10),
            Coin::<SuperGroupTestC1>::new(20),
        )
        .lossy_mul(d)
        .unwrap();
        assert_eq!(Duration::from_secs(5), res);
    }

    #[test]
    fn lossy_mul_max() {
        let d = Duration::from_secs(10);
        let res_x = SimpleFraction::new(
            Coin::<SuperGroupTestC1>::new(u128::MAX),
            Coin::<SuperGroupTestC1>::new(u128::MAX / 2),
        );
        let res = res_x.lossy_mul(d).unwrap();
        assert_eq!(Duration::from_secs(20), res);
    }
}
