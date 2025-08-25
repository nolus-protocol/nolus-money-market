use bnum::types::U256;

use crate::{
    coin::{Amount, Coin},
    fractionable::{HigherRank, ToPrimitive, TryFromPrimitive},
};

// TODO: Remove with Fragmentable
impl<U, C> HigherRank<U> for Coin<C>
where
    U: Into<Amount>,
{
    type Type = U256;

    type Intermediate = u128;
}

impl<C> ToPrimitive<U256> for Coin<C> {
    fn into_primitive(self) -> U256 {
        self._amount().into()
    }
}

impl<C> TryFromPrimitive<U256> for Coin<C> {
    fn try_from_primitive(primitive: U256) -> Option<Self> {
        Amount::try_from(primitive)
            .ok()
            .map(|amount| Coin::<C>::new(amount))
    }
}

// TODO remove when remove safe_mul() implementation
impl<C> From<Coin<C>> for U256 {
    fn from(coin: Coin<C>) -> Self {
        coin.into_primitive()
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
    };

    #[test]
    fn safe_mul() {
        use crate::fractionable::Fragmentable;
        assert_eq!(
            Coin::<SuperGroupTestC1>::new(30),
            Percent100::from_percent(100).of(Coin::<SuperGroupTestC1>::new(30))
        );

        assert_eq!(
            Coin::<SuperGroupTestC1>::new(4),
            Fragmentable::<u32>::safe_mul(
                Coin::<SuperGroupTestC1>::new(6),
                &SimpleFraction::new(2u32, 3u32)
            )
        );

        assert_eq!(
            Coin::<SuperGroupTestC1>::new(Amount::from(u32::MAX - 1)),
            Fragmentable::<u32>::safe_mul(
                Coin::<SuperGroupTestC1>::new(Amount::from(u32::MAX)),
                &SimpleFraction::new(u32::MAX - 1, u32::MAX)
            )
        );
    }
}
