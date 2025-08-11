use bnum::types::U256;
use sdk::cosmwasm_std::{Uint128, Uint256};

use crate::{
    coin::{Amount, Coin},
    duration::Duration,
    fractionable::{Fractionable, ToPrimitive, TryFromPrimitive},
    percent::Units as PercentUnits,
};

use super::HigherRank;

// TODO: Remove when refactor the Ord for Price
impl<U, C> HigherRank<U> for Coin<C>
where
    U: Into<Amount>,
{
    type Type = Uint256;

    type Intermediate = Uint128;
}

impl<C> Fractionable<Duration> for Coin<C> {
    type HigherPrimitive = U256;
}

// TODO: Remove when refactor the Ord for Price
impl<C> From<Coin<C>> for Uint256 {
    fn from(coin: Coin<C>) -> Self {
        Amount::from(coin).into()
    }
}

// TODO: Remove when refactor the Ord for Price
impl<C> From<Uint128> for Coin<C> {
    fn from(amount: Uint128) -> Self {
        let c: Amount = amount.into();
        c.into()
    }
}

impl<C, QuoteC> Fractionable<Coin<C>> for Coin<QuoteC> {
    type HigherPrimitive = U256;
}

impl<C> Fractionable<PercentUnits> for Coin<C> {
    type HigherPrimitive = U256;
}

impl<C> ToPrimitive<U256> for Coin<C> {
    fn into_primitive(self) -> U256 {
        self.amount().into()
    }
}

impl<C> TryFromPrimitive<U256> for Coin<C> {
    fn try_from_primitive(primitive: U256) -> Option<Self> {
        Amount::try_from(primitive)
            .ok()
            .map(|amount| Coin::<C>::new(amount))
    }
}

#[cfg(test)]
mod test {
    use currency::test::SuperGroupTestC1;

    use crate::{
        coin::{Amount, Coin},
        fraction::Fraction,
        percent::Percent100,
        ratio::SimpleFraction,
        rational::Rational,
    };

    #[test]
    fn of() {
        assert_eq!(
            Coin::<SuperGroupTestC1>::new(30),
            Percent100::from_percent(100).of(Coin::<SuperGroupTestC1>::new(30))
        );

        assert_eq!(
            Coin::<SuperGroupTestC1>::new(4),
            SimpleFraction::new(2u32, 3u32)
                .of(Coin::<SuperGroupTestC1>::new(6))
                .unwrap()
        );

        assert_eq!(
            Coin::<SuperGroupTestC1>::new(Amount::from(u32::MAX - 1)),
            SimpleFraction::new(u32::MAX - 1, u32::MAX)
                .of(Coin::<SuperGroupTestC1>::new(Amount::from(u32::MAX)))
                .unwrap()
        );
    }
}
