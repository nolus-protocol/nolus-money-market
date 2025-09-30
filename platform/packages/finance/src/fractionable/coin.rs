use bnum::types::U256;

use crate::{
    coin::{Amount, Coin},
    duration::Duration,
    fractionable::{CommonDoublePrimitive, Fractionable, IntoMax, ToDoublePrimitive, TryFromMax},
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

    fn to_double(self) -> Self::Double {
        U256::from(self)
    }
}

impl<C> CommonDoublePrimitive<Duration> for Coin<C> {
    type CommonDouble = U256;
}

impl<C> Fractionable<Duration> for Coin<C> {}

impl<C> IntoMax<U256> for Coin<C> {
    fn into(self) -> U256 {
        self.to_double()
    }
}

impl<C> TryFromMax<U256> for Coin<C> {
    fn try_from(max: U256) -> Option<Self> {
        max.try_into().ok().map(Coin::new)
    }
}

#[cfg(test)]
mod test {
    use currency::test::SuperGroupTestC1;

    use crate::{
        coin::{Amount, Coin},
        fractionable::FractionableLegacy,
        percent::Percent,
        ratio::SimpleFraction,
    };

    #[test]
    fn safe_mul() {
        assert_eq!(
            Coin::<SuperGroupTestC1>::new(30),
            Coin::<SuperGroupTestC1>::new(3).safe_mul(&Percent::from_percent(1000))
        );

        assert_eq!(
            Coin::<SuperGroupTestC1>::new(1000),
            FractionableLegacy::<u32>::safe_mul(
                Coin::<SuperGroupTestC1>::new(2),
                &SimpleFraction::new(1000u32, 2u32)
            )
        );

        assert_eq!(
            Coin::<SuperGroupTestC1>::new(2 * Amount::from(u32::MAX)),
            FractionableLegacy::<u32>::safe_mul(
                Coin::<SuperGroupTestC1>::new(2),
                &SimpleFraction::new(u32::MAX, 1u32)
            )
        );
    }
}
