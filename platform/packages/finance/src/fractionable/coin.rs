use bnum::types::U256;

use crate::{
    coin::{Amount, Coin},
    duration::Duration,
    fractionable::{CommonDoublePrimitive, Fractionable, IntoMax, ToDoublePrimitive, TryFromMax},
    percent::{Units as PercentUnits, bound::BoundPercent},
};

use super::HigherRank;

impl<U, C> HigherRank<U> for Coin<C>
where
    U: Into<Amount>,
{
    type Type = U256;
}

// TODO remove when FractionableLegacy usages are replaced
impl<C> From<Coin<C>> for U256 {
    fn from(coin: Coin<C>) -> Self {
        let c = Amount::from(coin);
        c.into()
    }
}

// TODO remove when FractionableLegacy usages are replaced
impl<C> TryInto<Coin<C>> for U256 {
    type Error = <u128 as TryFrom<U256>>::Error;

    fn try_into(self) -> Result<Coin<C>, Self::Error> {
        self.try_into().map(Coin::new)
    }
}

impl<C> ToDoublePrimitive for Coin<C> {
    type Double = U256;

    fn to_double(&self) -> Self::Double {
        self.amount().into()
    }
}

impl<C> CommonDoublePrimitive<Duration> for Coin<C> {
    type CommonDouble = <Self as ToDoublePrimitive>::Double;
}

impl<C, const UPPER_BOUND: PercentUnits> CommonDoublePrimitive<BoundPercent<UPPER_BOUND>>
    for Coin<C>
{
    type CommonDouble = <Self as ToDoublePrimitive>::Double;
}

impl<C> CommonDoublePrimitive<Self> for Coin<C> {
    type CommonDouble = <Self as ToDoublePrimitive>::Double;
}

impl<C> Fractionable<Duration> for Coin<C> {}

impl<C, const UPPER_BOUND: PercentUnits> Fractionable<BoundPercent<UPPER_BOUND>> for Coin<C> {}

impl<C> Fractionable<Self> for Coin<C> {}

impl<C> IntoMax<U256> for Coin<C> {
    fn into_max(self) -> U256 {
        self.to_double()
    }
}

impl<C> TryFromMax<U256> for Coin<C> {
    fn try_from_max(max: U256) -> Option<Self> {
        max.try_into().ok().map(Coin::new)
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
