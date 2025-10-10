use bnum::types::U256;

use crate::{
    coin::Coin,
    duration::{Duration, Units},
    fractionable::{
        CommonDoublePrimitive, Fractionable, HigherRank, IntoMax, ToDoublePrimitive, TryFromMax,
    },
};

impl<T> HigherRank<T> for u128
where
    T: Into<Self>,
{
    type Type = U256;
}

impl ToDoublePrimitive for Duration {
    type Double = u128;

    fn to_double(&self) -> Self::Double {
        self.nanos().into()
    }
}

impl<C> CommonDoublePrimitive<Coin<C>> for Duration {
    type CommonDouble = <Coin<C> as ToDoublePrimitive>::Double;
}

impl<C> Fractionable<Coin<C>> for Duration {}

impl IntoMax<U256> for Duration {
    fn into_max(self) -> U256 {
        self.to_double().into()
    }
}

impl TryFromMax<U256> for Duration {
    fn try_from_max(max: U256) -> Option<Self> {
        u128::try_from(max)
            .ok()
            .and_then(|u_128| Units::try_from(u_128).ok().map(Self::from_nanos))
    }
}

#[cfg(test)]
mod tests {

    use crate::{duration::Duration, ratio::SimpleFraction, rational::Rational, test::coin};

    #[test]
    fn of() {
        let d = Duration::from_secs(10);
        let res = SimpleFraction::new(coin::coin1(10), coin::coin1(20))
            .of(d)
            .unwrap();
        assert_eq!(Duration::from_secs(5), res);
    }

    #[test]
    fn of_max() {
        let d = Duration::from_secs(10);
        let res = SimpleFraction::new(coin::coin1(u128::MAX), coin::coin1(u128::MAX / 2))
            .of(d)
            .unwrap();

        assert_eq!(Duration::from_secs(20), res);
    }
}
