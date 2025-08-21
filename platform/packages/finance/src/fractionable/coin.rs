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
        Coin::new(amount.into())
    }
}
#[cfg(test)]
mod test {

    use crate::{coin::Amount, percent::Percent, ratio::Rational, test::coin};

    #[test]
    fn safe_mul() {
        use crate::fractionable::Fractionable;
        assert_eq!(
            coin::coin1(30),
            coin::coin1(3).safe_mul(&Percent::from_percent(1000))
        );

        assert_eq!(
            coin::coin1(1000),
            Fractionable::<u32>::safe_mul(coin::coin1(2), &Rational::new(1000u32, 2u32))
        );

        assert_eq!(
            coin::coin1(2 * Amount::from(u32::MAX)),
            Fractionable::<u32>::safe_mul(coin::coin1(2), &Rational::new(u32::MAX, 1u32))
        );
    }
}
