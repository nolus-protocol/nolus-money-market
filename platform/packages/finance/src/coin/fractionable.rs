use bnum::types::U256;

use crate::{
    coin::{Amount, Coin},
    duration::Duration,
    fractionable::{CommonDoublePrimitive, Fractionable, IntoMax, ToDoublePrimitive, TryFromMax},
    percent::{Units as PercentUnits, bound::BoundPercent},
};

pub(crate) type DoubleCoinPrimitive = U256;

impl<C> CommonDoublePrimitive<Duration> for Coin<C> {
    type CommonDouble = DoubleCoinPrimitive;
}

impl<C, const UPPER_BOUND: PercentUnits> CommonDoublePrimitive<BoundPercent<UPPER_BOUND>>
    for Coin<C>
{
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

impl<C, const UPPER_BOUND: PercentUnits> Fractionable<BoundPercent<UPPER_BOUND>> for Coin<C> {}

impl<C, Q> Fractionable<Coin<Q>> for Coin<C> {}

// Since both Duration and BoundPercent may appear as generic parameters,
// ToDoublePrimitive is used instead of CommonDoublePrimitive to avoid ambiguity
impl<C> IntoMax<DoubleCoinPrimitive> for Coin<C> {
    fn into_max(self) -> DoubleCoinPrimitive {
        self.to_double()
    }
}

impl<C> ToDoublePrimitive for Coin<C> {
    type Double = DoubleCoinPrimitive;

    fn to_double(&self) -> Self::Double {
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
        coin::Amount, percent::Percent, ratio::SimpleFraction, rational::Rational, test::coin,
    };

    #[test]
    fn of() {
        assert_eq!(
            coin::coin1(30),
            Percent::from_percent(1000).of(coin::coin1(3)).unwrap()
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
