use bnum::types::U256;

use crate::coin::{Amount, Coin};

use super::HigherRank;

impl<U, C> HigherRank<U> for Coin<C>
where
    U: Into<Amount>,
{
    type Type = U256;

    type Intermediate = u128;
}

impl<C> From<Coin<C>> for U256 {
    fn from(coin: Coin<C>) -> Self {
        let c: Amount = coin.into();
        c.into()
    }
}

#[cfg(test)]
mod test {
    use currency::test::SuperGroupTestC1;

    use crate::{
        coin::{Amount, Coin},
        percent::Percent100,
        ratio::SimpleFraction,
    };

    #[test]
    fn safe_mul() {
        use crate::fractionable::Fractionable;
        assert_eq!(
            Coin::<SuperGroupTestC1>::new(30),
            Coin::<SuperGroupTestC1>::new(30).safe_mul(&Percent100::from_percent(100))
        );

        assert_eq!(
            Coin::<SuperGroupTestC1>::new(4),
            Fractionable::<u32>::safe_mul(
                Coin::<SuperGroupTestC1>::new(6),
                &SimpleFraction::new(2u32, 3u32)
            )
        );

        assert_eq!(
            Coin::<SuperGroupTestC1>::new(Amount::from(u32::MAX - 1)),
            Fractionable::<u32>::safe_mul(
                Coin::<SuperGroupTestC1>::new(Amount::from(u32::MAX)),
                &SimpleFraction::new(u32::MAX - 1, u32::MAX)
            )
        );
    }
}
