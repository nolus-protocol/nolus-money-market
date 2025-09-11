use std::ops::{Div, Rem};

use bnum::types::U256;
use gcd::Gcd;

use crate::{
    coin::{Amount, Coin},
    fractionable::scalar::Scalar,
};

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

impl<C> Scalar for Coin<C> {
    type Times = Amount;

    fn gcd(self, other: Self) -> Self::Times {
        Gcd::gcd(self.amount(), other.amount())
    }

    fn scale_up(self, scale: Self::Times) -> Option<Self> {
        self.amount().checked_mul(scale).map(Self::new)
    }

    fn scale_down(self, scale: Self::Times) -> Self {
        debug_assert_ne!(scale, 0);

        Self::new(self.amount().div(scale))
    }

    fn modulo(self, scale: Self::Times) -> Self::Times {
        self.amount().rem(scale)
    }

    fn into_times(self) -> Self::Times {
        self.amount()
    }
}

impl Scalar for Amount {
    type Times = Self;

    fn gcd(self, other: Self) -> Self::Times {
        Gcd::gcd(self, other)
    }

    fn scale_up(self, scale: Self::Times) -> Option<Self> {
        self.checked_mul(scale)
    }

    fn scale_down(self, scale: Self::Times) -> Self {
        debug_assert_ne!(scale, 0);

        self.div(scale)
    }

    fn modulo(self, scale: Self::Times) -> Self::Times {
        debug_assert_ne!(scale, 0);

        self.rem(scale)
    }

    fn into_times(self) -> Self::Times {
        self
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
            Coin::<SuperGroupTestC1>::new(3).safe_mul(&Percent::from_percent(1000))
        );

        assert_eq!(
            Coin::<SuperGroupTestC1>::new(1000),
            Fractionable::<u32>::safe_mul(
                Coin::<SuperGroupTestC1>::new(2),
                &Rational::new(Percent::from_permille(1000), Percent::from_permille(2))
            )
        );

        assert_eq!(
            Coin::<SuperGroupTestC1>::new(2 * Amount::from(u32::MAX)),
            Fractionable::<u32>::safe_mul(
                Coin::<SuperGroupTestC1>::new(2),
                &Rational::new(Percent::from_permille(u32::MAX), Percent::from_permille(1))
            )
        );
    }
}
