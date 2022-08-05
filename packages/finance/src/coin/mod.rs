mod coinc;
#[cfg(feature = "testing")]
pub use coinc::funds;
pub use coinc::CoinDTO;
mod serde;

use std::{
    fmt::{Debug, Display, Formatter, Write},
    marker::PhantomData,
    ops::{Add, Div, Sub, SubAssign},
};
use std::ops::AddAssign;

use ::serde::{Deserialize, Serialize};
use schemars::JsonSchema;

use gcd::Gcd;

use crate::currency::Currency;

pub type Amount = u128;

#[derive(
    Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default, Serialize, Deserialize, JsonSchema,
)]
pub struct Coin<C>
where
    C: Currency,
{
    amount: Amount,
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
    pub fn new(amount: Amount) -> Self {
        Self {
            amount,
            symbol: PhantomData::<C>,
        }
    }

    pub fn is_zero(&self) -> bool {
        self.amount == Amount::default()
    }

    pub(super) fn into_coprime_with<OtherC>(self, other: Coin<OtherC>) -> (Self, Coin<OtherC>)
    where
        OtherC: Currency,
    {
        debug_assert!(!self.is_zero() && !other.is_zero());
        let gcd = self.amount.gcd(other.amount);
        debug_assert!(gcd > 0);

        debug_assert_eq!(self.amount % gcd, 0);
        debug_assert_eq!(other.amount % gcd, 0);
        (
            Self::new(self.amount / gcd),
            Coin::<OtherC>::new(other.amount / gcd),
        )
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

impl<C> AddAssign<Coin<C>> for Coin<C>
where
    C: Currency,
{
    fn add_assign(&mut self, rhs: Coin<C>) {
        self.amount += rhs.amount;
    }
}

impl<C> SubAssign<Coin<C>> for Coin<C>
where
    C: Currency,
{
    fn sub_assign(&mut self, rhs: Coin<C>) {
        self.amount -= rhs.amount;
    }
}

impl<C> Div<Amount> for Coin<C>
where
    C: Currency,
{
    type Output = Self;

    fn div(self, rhs: Amount) -> Self::Output {
        Self::Output {
            amount: self.amount / rhs,
            symbol: self.symbol,
        }
    }
}

impl<C> Display for Coin<C>
where
    C: Currency,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.amount.to_string())?;
        f.write_char(' ')?;
        f.write_str(C::SYMBOL)?;
        Ok(())
    }
}

impl<C> From<Amount> for Coin<C>
where
    C: Currency,
{
    fn from(amount: Amount) -> Self {
        Self::new(amount)
    }
}

impl<C> From<Coin<C>> for Amount
where
    C: Currency,
{
    fn from(coin: Coin<C>) -> Self {
        coin.amount
    }
}

#[cfg(test)]
mod test {

    use crate::{
        currency::{Nls, Usdc},
        percent::test::test_of,
    };

    use super::{Amount, Coin};

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
        test_of(1000, usdc(Amount::MAX), usdc(Amount::MAX));
    }

    #[test]
    fn is_zero() {
        assert!(usdc(0).is_zero());
        assert!(!usdc(1).is_zero());
    }

    #[test]
    #[should_panic]
    fn of_overflow() {
        let max_amount = usdc(Amount::MAX);
        test_of(1001, max_amount, max_amount);
    }

    #[test]
    fn div() {
        assert_eq!(usdc(18 / 3), usdc(18) / 3);
        assert_eq!(usdc(0), usdc(0) / 5);
        assert_eq!(usdc(17 / 3), usdc(17) / 3);
    }

    #[test]
    fn div_ceil() {
        assert_eq!(usdc(17 / 3), usdc(17) / 3);
    }

    #[test]
    fn coprime() {
        coprime_impl(1, 1, 2);
        coprime_impl(1, 5, 7);
        coprime_impl(6, 18, 12);
        coprime_impl(6, 12, 18);
        coprime_impl(13, 13, 13);
        coprime_impl(13, 13, 26);
        coprime_impl(Amount::MAX, Amount::MAX, Amount::MAX);
    }

    fn coprime_impl(gcd: Amount, a1: Amount, a2: Amount) {
        assert_eq!(
            (usdc(a1 / gcd), nls(a2 / gcd)),
            usdc(a1).into_coprime_with(nls(a2))
        );
        assert_eq!(
            (nls(a1 / gcd), nls(a2 / gcd)),
            nls(a1).into_coprime_with(nls(a2))
        );
    }

    fn usdc(amount: Amount) -> Coin<Usdc> {
        Coin::new(amount)
    }

    fn nls(amount: Amount) -> Coin<Nls> {
        Coin::new(amount)
    }
}
