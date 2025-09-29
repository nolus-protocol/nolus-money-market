use bnum::types::U256;

use crate::{
    coin::Coin,
    duration::{Duration, Units},
    fractionable::{CommonDoublePrimitive, Fractionable, IntoMax, ToDoublePrimitive, TryFromMax},
    ratio::RatioLegacy,
};

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
        let d128 = u128::from(self);
        // TODO re-assess the design of Ratio ... and whether it could be > 1
        d128.safe_mul(fraction).try_into().expect(
            "TODO remove when refactor Fractionable. Overflow computing a fraction of duration",
        )
    }
}

impl ToDoublePrimitive for Duration {
    type Double = u128;

    fn to_double(self) -> Self::Double {
        self.nanos().into()
    }
}

impl<C> CommonDoublePrimitive<Coin<C>> for Duration {
    type CommonDouble = U256;
}

impl<C> Fractionable<Coin<C>> for Duration {}

impl IntoMax<U256> for Duration {
    fn into(self) -> U256 {
        self.to_double().into()
    }
}

impl TryFromMax<U256> for Duration {
    fn try_from(max: U256) -> Option<Self> {
        u128::try_from(max)
            .ok()
            .and_then(|u_128| Units::try_from(u_128).ok().map(Duration::from_nanos))
    }
}

#[cfg(test)]
mod tests {
    use currency::test::SuperGroupTestC1;

    use crate::{
        coin::Coin, duration::Duration, fractionable::FractionableLegacy, ratio::SimpleFraction,
    };

    #[test]
    fn safe_mul() {
        let d = Duration::from_secs(10);
        let res = d.safe_mul(&SimpleFraction::new(
            Coin::<SuperGroupTestC1>::new(10),
            Coin::<SuperGroupTestC1>::new(20),
        ));
        assert_eq!(Duration::from_secs(5), res);
    }

    #[test]
    fn safe_mul_max() {
        let d = Duration::from_secs(10);
        let res = d.safe_mul(&SimpleFraction::new(
            Coin::<SuperGroupTestC1>::new(u128::MAX),
            Coin::<SuperGroupTestC1>::new(u128::MAX / 2),
        ));

        assert_eq!(Duration::from_secs(20), res);
    }
}
