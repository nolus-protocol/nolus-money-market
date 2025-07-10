use bnum::types::U256;

use crate::coin::{Amount, Coin};

use super::HigherRank;

impl<U, C> HigherRank<U> for Coin<C>
where
    U: Into<Amount>,
{
    type Type = U256;
}

impl<C> From<Coin<C>> for U256 {
    fn from(coin: Coin<C>) -> Self {
        let c: Amount = coin.into();
        c.into()
    }
}

impl<C> TryInto<Coin<C>> for U256 {
    type Error = <u128 as TryFrom<U256>>::Error;

    fn try_into(self) -> Result<Coin<C>, Self::Error> {
        self.try_into().map(Coin::new)
    }
}

#[cfg(test)]
mod test {
    use currency::test::SuperGroupTestC1;

    use crate::{
        coin::{Amount, Coin},
        percent::Percent,
        ratio::SimpleFraction,
    };

    #[test]
    fn safe_mul() {
        use crate::fractionable::Fractionable;
        assert_eq!(
            Coin::<SuperGroupTestC1>::new(30),
            Coin::<SuperGroupTestC1>::new(3).safe_mul(&Percent::from_percent(1000))
        );

        assert_eq!(
            Coin::<SuperGroupTestC1>::new(1000),
            Fractionable::<u32>::safe_mul(
                Coin::<SuperGroupTestC1>::new(2),
                &SimpleFraction::new(1000u32, 2u32)
            )
        );

        assert_eq!(
            Coin::<SuperGroupTestC1>::new(2 * Amount::from(u32::MAX)),
            Fractionable::<u32>::safe_mul(
                Coin::<SuperGroupTestC1>::new(2),
                &SimpleFraction::new(u32::MAX, 1u32)
            )
        );
    }
}
