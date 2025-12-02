use bnum::types::U256;

use crate::{
    coin::{Amount, Coin},
    duration::Duration,
    fractionable::{
        CommonDoublePrimitive, Fractionable, HigherRank, IntoMax, ToDoublePrimitive, TryFromMax,
    },
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

impl<C> CommonDoublePrimitive<Self> for Coin<C> {
    type CommonDouble = DoubleCoinPrimitive;
}

// TODO remove this implemenatation when Price converts to SimpleFraction<Quote, C>
impl<C> CommonDoublePrimitive<u128> for Coin<C> {
    type CommonDouble = <Self as ToDoublePrimitive>::Double;
}

impl<C> Fractionable<Duration> for Coin<C> {}

impl<C, const UPPER_BOUND: PercentUnits> Fractionable<BoundPercent<UPPER_BOUND>> for Coin<C> {}

impl<C> Fractionable<Self> for Coin<C> {}

// TODO remove this implemenatation when Price converts to SimpleFraction<Quote, C>
impl<C> Fractionable<u128> for Coin<C> {}

// TODO remove when FractionableLegacy usages are replaced
impl<C> From<Coin<C>> for DoubleCoinPrimitive {
    fn from(coin: Coin<C>) -> Self {
        Amount::from(coin).into()
    }
}

impl<U, C> HigherRank<U> for Coin<C>
where
    U: Into<Amount>,
{
    type Type = DoubleCoinPrimitive;
}

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

// TODO remove when FractionableLegacy usages are replaced
impl<C> TryInto<Coin<C>> for DoubleCoinPrimitive {
    type Error = <u128 as TryFrom<DoubleCoinPrimitive>>::Error;

    fn try_into(self) -> Result<Coin<C>, Self::Error> {
        self.try_into().map(Coin::new)
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
