use sdk::cosmwasm_std::{Uint128, Uint256};

use crate::{
    coin::{Amount, Coin},
    currency::Currency,
};

use super::HigherRank;

impl<U, C> HigherRank<U> for Coin<C>
where
    U: Into<Amount>,
    C: Currency,
{
    type Type = Uint256;

    type Intermediate = Uint128;
}

impl<C> From<Coin<C>> for Uint256
where
    C: Currency,
{
    fn from(coin: Coin<C>) -> Self {
        let c: Amount = coin.into();
        c.into()
    }
}

impl<C> From<Uint128> for Coin<C>
where
    C: Currency,
{
    fn from(amount: Uint128) -> Self {
        let c: Amount = amount.into();
        c.into()
    }
}
#[cfg(test)]
mod test {
    use crate::{
        coin::{Amount, Coin},
        percent::Percent,
        ratio::Rational,
        test::currency::Nls,
    };

    #[test]
    fn safe_mul() {
        use crate::fractionable::Fractionable;
        assert_eq!(
            Coin::<Nls>::new(30),
            Coin::<Nls>::new(3).safe_mul(&Percent::from_percent(1000))
        );

        assert_eq!(
            Coin::<Nls>::new(1000),
            <Coin::<Nls> as Fractionable<u32>>::safe_mul(
                Coin::<Nls>::new(2),
                &Rational::new(1000u32, 2u32)
            )
        );

        assert_eq!(
            Coin::<Nls>::new(2 * Amount::from(u32::MAX)),
            <Coin::<Nls> as Fractionable<u32>>::safe_mul(
                Coin::<Nls>::new(2),
                &Rational::new(u32::MAX, 1u32)
            )
        );
    }
}
