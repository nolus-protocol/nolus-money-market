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

    use crate::{coin::Coin, percent::Percent, ratio::Ratio};

    #[test]
    fn safe_mul() {
        use crate::fractionable::Fractionable;
        assert_eq!(
            Coin::<SuperGroupTestC1>::new(30),
            Coin::<SuperGroupTestC1>::new(3).safe_mul(&Percent::from_percent(1000).into())
        );

        assert_eq!(
            Coin::<SuperGroupTestC1>::new(4),
            Fractionable::<u32>::safe_mul(
                Coin::<SuperGroupTestC1>::new(6),
                &Ratio::new(2u32, 3u32)
            )
        );

        assert_eq!(
            Coin::<SuperGroupTestC1>::new(2u32.into()),
            Fractionable::<u32>::safe_mul(
                Coin::<SuperGroupTestC1>::new(4),
                &Ratio::new(u32::MAX / 2, u32::MAX)
            )
        );
    }
}
