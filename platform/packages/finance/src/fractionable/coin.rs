use sdk::cosmwasm_std::{Uint128, Uint256};

use crate::coin::{Amount, Coin};

use super::HigherRank;

impl<U, C> HigherRank<U> for Coin<C>
where
    U: Into<Amount>,
{
    type Type = Uint256;

    type Intermediate = Uint128;
}

impl<C> From<Coin<C>> for Uint256 {
    fn from(coin: Coin<C>) -> Self {
        let c: Amount = coin.into();
        c.into()
    }
}

impl<C> From<Uint128> for Coin<C> {
    fn from(amount: Uint128) -> Self {
        let c: Amount = amount.into();
        c.into()
    }
}
#[cfg(test)]
mod test {
    use currency::test::SuperGroupTestC1;

    use crate::{
        coin::{Amount, Coin},
        percent::Percent,
        ratio::Rational,
    };

    #[test]
    fn safe_mul() {
        use crate::fractionable::Fractionable;
        assert_eq!(
            Coin::<SuperGroupTestC1>::new(30),
            Coin::<SuperGroupTestC1>::new(3).safe_mul(&Percent::from_percent(1000).into())
        );

        assert_eq!(
            Coin::<SuperGroupTestC1>::new(1000),
            Fractionable::<u32>::safe_mul(
                Coin::<SuperGroupTestC1>::new(2),
                &Rational::new(1000u32, 2u32).into()
            )
        );

        assert_eq!(
            Coin::<SuperGroupTestC1>::new(2 * Amount::from(u32::MAX)),
            Fractionable::<u32>::safe_mul(
                Coin::<SuperGroupTestC1>::new(2),
                &Rational::new(u32::MAX, 1u32).into()
            )
        );
    }
}
