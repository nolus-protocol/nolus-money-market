use std::ops::{Div, Rem};

use bnum::types::U256;
use gcd::Gcd;

use crate::{
    coin::Coin,
    duration::{Duration, Units},
    fractionable::scalar::Scalar,
    ratio::RatioLegacy,
};

use super::{Fractionable, HigherRank};

impl<T> HigherRank<T> for u128
where
    T: Into<Self>,
{
    type Type = U256;
}

impl<C> Fractionable<Coin<C>> for Duration {
    #[track_caller]
    fn safe_mul<F>(self, fraction: &F) -> Self
    where
        F: RatioLegacy<Coin<C>>,
    {
        let d128: u128 = self.into();
        // TODO re-assess the design of Ratio ... and whether it could be > 1
        d128.safe_mul(fraction)
            .try_into()
            .expect("overflow computing a fraction of duration")
    }
}

impl Scalar for Units {
    type Times = Self;

    fn gcd(self, other: Self) -> Self::Times {
        Gcd::gcd(self, other)
    }

    fn scale_up(self, scale: Self::Times) -> Option<Self> {
        self.checked_mul(scale)
    }

    fn scale_down(self, scale: Self::Times) -> Self {
        debug_assert_ne!(scale, 0);

        self.div(scale)
    }

    fn modulo(self, scale: Self::Times) -> Self::Times {
        debug_assert_ne!(scale, 0);

        self.rem(scale)
    }

    fn into_times(self) -> Self::Times {
        self
    }
}
#[cfg(test)]
mod tests {
    use currency::test::SuperGroupTestC1;

    use crate::{coin::Coin, duration::Duration, fractionable::Fractionable, ratio::Rational};

    #[test]
    fn safe_mul() {
        let d = Duration::from_secs(10);
        let res = d.safe_mul(&Rational::new(
            Coin::<SuperGroupTestC1>::new(10),
            Coin::<SuperGroupTestC1>::new(20),
        ));
        assert_eq!(Duration::from_secs(5), res);
    }

    #[test]
    fn safe_mul_max() {
        let d = Duration::from_secs(10);
        let res = d.safe_mul(&Rational::new(
            Coin::<SuperGroupTestC1>::new(u128::MAX),
            Coin::<SuperGroupTestC1>::new(u128::MAX / 2),
        ));
        assert_eq!(Duration::from_secs(20), res);
    }
}
