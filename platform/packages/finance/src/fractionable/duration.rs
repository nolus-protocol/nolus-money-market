use bnum::types::U256;

use crate::{
    coin::Coin,
    duration::{Duration, Units},
    fractionable::{CommonDoublePrimitive, Fractionable, IntoMax, ToDoublePrimitive, TryFromMax},
};

use super::HigherRank;

impl<T> HigherRank<T> for u128
where
    T: Into<Self>,
{
    type Type = U256;
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

    use crate::{coin::Coin, duration::Duration, ratio::SimpleFraction, rational::Rational};

    #[test]
    fn of() {
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
    fn of_max() {
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
