use bnum::types::U256;

use crate::{
    coin::{Amount, Coin},
    duration::Duration,
    fractionable::{CommonDoublePrimitive, Fractionable, IntoDoublePrimitive, IntoMax, TryFromMax},
    percent::permilles::Permilles,
};

pub(crate) type DoubleCoinPrimitive = U256;

impl<C> CommonDoublePrimitive<Duration> for Coin<C> {
    type CommonDouble = DoubleCoinPrimitive;
}

impl<C> CommonDoublePrimitive<Permilles> for Coin<C> {
    type CommonDouble = DoubleCoinPrimitive;
}

impl<C, Q> CommonDoublePrimitive<Coin<Q>> for Coin<C> {
    type CommonDouble = DoubleCoinPrimitive;
}

// TODO remove this implemenatation when Price converts to SimpleFraction<Quote, C>
impl<C> CommonDoublePrimitive<Amount> for Coin<C> {
    type CommonDouble = DoubleCoinPrimitive;
}

impl<C> Fractionable<Duration> for Coin<C> {}

impl<C> Fractionable<Permilles> for Coin<C> {}

impl<C, Q> Fractionable<Coin<Q>> for Coin<C> {}

impl<C> IntoMax<DoubleCoinPrimitive> for Coin<C> {
    fn into_max(self) -> DoubleCoinPrimitive {
        self.into_double()
    }
}

impl<C> IntoDoublePrimitive for Coin<C> {
    type Double = DoubleCoinPrimitive;

    fn into_double(self) -> Self::Double {
        self.amount.into()
    }
}

impl<C> TryFromMax<DoubleCoinPrimitive> for Coin<C> {
    fn try_from_max(max: DoubleCoinPrimitive) -> Option<Self> {
        max.try_into().map(Coin::new).ok()
    }
}

#[cfg(test)]
mod test {

    use crate::{
        coin::Amount,
        percent::{HUNDRED, Percent},
        ratio::SimpleFraction,
        rational::Rational,
        test::coin,
    };

    #[test]
    fn of() {
        assert_eq!(
            coin::coin1(30),
            Percent::from_percent(HUNDRED).of(coin::coin1(3)).unwrap()
        );

        assert_eq!(
            coin::coin1(1000),
            SimpleFraction::new(coin::coin1(1000), coin::coin1(2))
                .of(coin::coin1(2))
                .unwrap()
        );

        assert_eq!(
            coin::coin1(2 * Amount::from(u32::MAX)),
            SimpleFraction::new(coin::coin1(Amount::from(u32::MAX)), coin::coin1(1))
                .of(coin::coin1(2))
                .unwrap()
        );
    }
}
