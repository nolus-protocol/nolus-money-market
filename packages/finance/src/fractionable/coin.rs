use cosmwasm_std::{Uint128, Uint256};

use crate::coin::Coin;

use super::HigherRank;

impl<U, C> HigherRank<U> for Coin<C>
where
    U: Into<u128>,
{
    type Type = Uint256;

    type Intermediate = Uint128;
}

impl<C> From<Coin<C>> for Uint256 {
    fn from(coin: Coin<C>) -> Self {
        let c: u128 = coin.into();
        c.into()
    }
}

impl<C> From<Uint128> for Coin<C> {
    fn from(amount: Uint128) -> Self {
        let c: u128 = amount.into();
        c.into()
    }
}
#[cfg(test)]
mod test {
    use crate::{coin::Coin, currency::Nls, percent::Percent, ratio::Rational};

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
            Coin::<Nls>::new(2u128 * u128::from(u32::MAX)),
            <Coin::<Nls> as Fractionable<u32>>::safe_mul(
                Coin::<Nls>::new(2),
                &Rational::new(u32::MAX, 1u32)
            )
        );
    }
}
