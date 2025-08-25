use bnum::types::U256;

use crate::{coin::Coin, duration::Duration, ratio::Ratio};

use super::{Fractionable, HigherRank};

impl<T> HigherRank<T> for u128
where
    T: Into<Self>,
{
    type Type = U256;
    type Intermediate = u128;
}

impl<C> Fractionable<Coin<C>> for Duration {
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

    use crate::{coin::Coin, duration::Duration, ratio::SimpleFraction, rational::Rational};

    #[test]
    fn safe_mul() {
        let d = Duration::from_secs(10);
        let res = SimpleFraction::new(
            Coin::<SuperGroupTestC1>::new(10),
            Coin::<SuperGroupTestC1>::new(20),
        )
        .of(d)
        .unwrap();
        assert_eq!(Duration::from_secs(5), res);
    }

    #[test]
    fn safe_mul_max() {
        let d = Duration::from_secs(10);
        let res = SimpleFraction::new(
            Coin::<SuperGroupTestC1>::new(u128::MAX),
            Coin::<SuperGroupTestC1>::new(u128::MAX / 2),
        )
        .of(d)
        .unwrap();
        assert_eq!(Duration::from_secs(20), res);
    }
}
