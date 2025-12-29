use crate::{
    coin::{Coin, DoubleCoinPrimitive},
    duration::Duration,
    fractionable::{CommonDoublePrimitive, Fractionable, IntoMax, ToDoublePrimitive, TryFromMax},
};

type DoubleDurationPrimitive = u128;

impl<C> CommonDoublePrimitive<Coin<C>> for Duration {
    type CommonDouble = DoubleCoinPrimitive;
}

impl<C> Fractionable<Coin<C>> for Duration {}

impl IntoMax<DoubleCoinPrimitive> for Duration {
    fn into_max(self) -> DoubleCoinPrimitive {
        self.to_double().into()
    }
}

impl ToDoublePrimitive for Duration {
    type Double = DoubleDurationPrimitive;

    fn to_double(&self) -> Self::Double {
        self.nanos().into()
    }
}

impl TryFromMax<DoubleCoinPrimitive> for Duration {
    fn try_from_max(max: DoubleCoinPrimitive) -> Option<Self> {
        max.try_into().map(Self::from_nanos).ok()
    }
}

#[cfg(test)]
mod tests {

    use crate::{
        duration::{Duration, fractionable::DoubleDurationPrimitive},
        ratio::SimpleFraction,
        rational::Rational,
        test::coin,
    };

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
        let res = SimpleFraction::new(
            coin::coin1(DoubleDurationPrimitive::MAX),
            coin::coin1(DoubleDurationPrimitive::MAX / 2),
        )
        .of(d)
        .unwrap();

        assert_eq!(Duration::from_secs(20), res);
    }
}
