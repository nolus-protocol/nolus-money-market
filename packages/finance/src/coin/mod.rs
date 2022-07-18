mod coinc;
#[cfg(feature = "testing")]
pub use coinc::funds;
pub use coinc::CoinDTO;
mod serde;

use std::{
    fmt::{Debug, Display, Formatter, Write},
    marker::PhantomData,
    ops::{Add, Sub},
};

use ::serde::{Deserialize, Serialize};
use schemars::JsonSchema;

use crate::currency::Currency;

#[derive(
    Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default, Serialize, Deserialize, JsonSchema,
)]
pub struct Coin<C>
where
    C: Currency,
{
    amount: u128,
    // using `with` for both directions implies implementing JsonSchema for that type
    // https://github.com/GREsau/schemars/issues/89
    #[serde(serialize_with = "serde::serialize")]
    #[serde(deserialize_with = "serde::deserialize")]
    symbol: PhantomData<C>,
}

impl<C> Coin<C>
where
    C: Currency,
{
    pub fn new(amount: u128) -> Self {
        Self {
            amount,
            symbol: PhantomData::<C>,
        }
    }

    pub fn is_zero(&self) -> bool {
        self.amount == u128::default()
    }

    pub(super) fn amount(&self) -> u128 {
        self.amount
    }
}
impl<C> Add<Coin<C>> for Coin<C>
where
    C: Currency,
{
    type Output = Self;

    fn add(self, rhs: Coin<C>) -> Self::Output {
        Self::Output {
            amount: self.amount + rhs.amount,
            symbol: self.symbol,
        }
    }
}

impl<C> Sub<Coin<C>> for Coin<C>
where
    C: Currency,
{
    type Output = Self;

    fn sub(self, rhs: Coin<C>) -> Self::Output {
        Self::Output {
            amount: self.amount - rhs.amount,
            symbol: self.symbol,
        }
    }
}

impl<C> Display for Coin<C>
where
    C: Currency,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.amount().to_string())?;
        f.write_char(' ')?;
        f.write_str(C::SYMBOL)?;
        Ok(())
    }
}

impl<C> From<u128> for Coin<C>
where
    C: Currency,
{
    fn from(amount: u128) -> Self {
        Self::new(amount)
    }
}

impl<C> From<Coin<C>> for u128
where
    C: Currency,
{
    fn from(coin: Coin<C>) -> Self {
        coin.amount()
    }
}

#[cfg(test)]
mod test {

    use crate::{
        currency::{Nls, Usdc},
        percent::test::test_of,
    };

    use super::Coin;

    #[test]
    fn display() {
        assert_eq!("25 unls", nls(25).to_string());
        assert_eq!("0 uusdc", usdc(0).to_string());
    }

    #[test]
    fn of_are() {
        test_of(10, usdc(100), usdc(1));
        test_of(11, usdc(100), usdc(1));
        test_of(11, usdc(90), usdc(0));
        test_of(11, usdc(91), usdc(1));
        test_of(110, usdc(100), usdc(11));
        test_of(12, usdc(100), usdc(1));
        test_of(12, usdc(84), usdc(1));
        test_of(12, usdc(83), usdc(0));
        test_of(18, usdc(100), usdc(1));
        test_of(18, usdc(56), usdc(1));
        test_of(18, usdc(55), usdc(0));
        test_of(18, usdc(120), usdc(2));
        test_of(18, usdc(112), usdc(2));
        test_of(18, usdc(111), usdc(1));
        test_of(1000, usdc(u128::MAX), usdc(u128::MAX));
    }

    #[test]
    fn is_zero() {
        assert!(usdc(0).is_zero());
        assert!(!usdc(1).is_zero());
    }

    #[test]
    #[should_panic]
    fn of_overflow() {
        let max_amount = usdc(u128::MAX);
        test_of(1001, max_amount, max_amount);
    }
    fn usdc(amount: u128) -> Coin<Usdc> {
        Coin::new(amount)
    }

    fn nls(amount: u128) -> Coin<Nls> {
        Coin::new(amount)
    }
}
