use std::{
    fmt::{Debug, Display, Formatter},
    marker::PhantomData,
    ops::{Add, AddAssign, Sub, SubAssign},
};

use ::serde::{Deserialize, Serialize};

use currency::Currency;
use sdk::schemars::{self, JsonSchema};

use crate::zero::Zero;

pub use self::dto::{from_amount_ticker, CoinDTO, IntoDTO};

mod dto;
mod serde;

pub type Amount = u128;

#[derive(
    Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default, Serialize, Deserialize, JsonSchema,
)]
pub struct Coin<C> {
    amount: Amount,
    #[serde(skip)]
    ticker: PhantomData<C>,
}

impl<C> Coin<C>
where
    C: Currency,
{
    pub const fn new(amount: Amount) -> Self {
        Self {
            amount,
            ticker: PhantomData,
        }
    }

    pub const fn is_zero(&self) -> bool {
        self.amount == Zero::ZERO
    }

    #[track_caller]
    pub fn checked_add(self, rhs: Self) -> Option<Self> {
        let may_amount = self.amount.checked_add(rhs.amount);
        may_amount.map(|amount| Self {
            amount,
            ticker: self.ticker,
        })
    }

    #[track_caller]
    pub fn saturating_sub(self, rhs: Self) -> Self {
        self.amount.saturating_sub(rhs.amount).into()
    }

    #[track_caller]
    pub fn checked_mul(self, rhs: Amount) -> Option<Self> {
        let may_amount = self.amount.checked_mul(rhs);
        may_amount.map(|amount| Self {
            amount,
            ticker: self.ticker,
        })
    }

    #[track_caller]
    pub fn checked_div(self, rhs: Amount) -> Option<Self> {
        let may_amount = self.amount.checked_div(rhs);
        may_amount.map(|amount| Self {
            amount,
            ticker: self.ticker,
        })
    }

    #[track_caller]
    pub(super) const fn into_coprime_with<OtherC>(self, other: Coin<OtherC>) -> (Self, Coin<OtherC>)
    where
        OtherC: Currency,
    {
        debug_assert!(!self.is_zero(), "LHS-value's amount is zero!");
        debug_assert!(!other.is_zero(), "RHS-value's amount is zero!");

        let gcd: Amount = gcd::binary_u128(self.amount, other.amount);

        debug_assert!(gcd > 0);

        debug_assert!(
            self.amount % gcd == 0,
            "LHS-value's amount is not divisible by the GCD!"
        );
        debug_assert!(
            other.amount % gcd == 0,
            "RHS-value's amount is not divisible by the GCD!"
        );

        (
            Self::new(self.amount / gcd),
            Coin::<OtherC>::new(other.amount / gcd),
        )
    }
}

impl<C> Zero for Coin<C>
where
    C: Currency,
{
    const ZERO: Self = Self::new(Zero::ZERO);
}

impl<C> Add<Coin<C>> for Coin<C>
where
    C: Currency,
{
    type Output = Self;

    #[track_caller]
    fn add(self, rhs: Coin<C>) -> Self::Output {
        self.checked_add(rhs)
            .expect("addition should not overflow with real data")
    }
}

impl<C> Sub<Coin<C>> for Coin<C>
where
    C: Currency,
{
    type Output = Self;

    #[track_caller]
    fn sub(mut self, rhs: Coin<C>) -> Self::Output {
        self -= rhs;
        self
    }
}

impl<C> AddAssign<Coin<C>> for Coin<C>
where
    C: Currency,
{
    #[track_caller]
    fn add_assign(&mut self, rhs: Coin<C>) {
        self.amount += rhs.amount;
    }
}

impl<C> SubAssign<Coin<C>> for Coin<C>
where
    C: Currency,
{
    #[track_caller]
    fn sub_assign(&mut self, rhs: Coin<C>) {
        self.amount -= rhs.amount;
    }
}

impl<C> Display for Coin<C>
where
    C: Currency,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{} {}", self.amount, C::TICKER))
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

pub type WithCoinResult<V> = Result<<V as WithCoin>::Output, <V as WithCoin>::Error>;

pub trait WithCoin {
    type Output;
    type Error;

    fn on<C>(&self, coin: Coin<C>) -> WithCoinResult<Self>
    where
        C: Currency;
}

impl<T> WithCoin for &'_ T
where
    T: WithCoin,
{
    type Output = T::Output;
    type Error = T::Error;

    fn on<C>(&self, coin: Coin<C>) -> WithCoinResult<Self>
    where
        C: Currency,
    {
        T::on(self, coin)
    }
}

impl<T> WithCoin for &'_ mut T
where
    T: WithCoin,
{
    type Output = T::Output;
    type Error = T::Error;

    fn on<C>(&self, coin: Coin<C>) -> WithCoinResult<Self>
    where
        C: Currency,
    {
        T::on(self, coin)
    }
}

#[cfg(test)]
mod test {
    use crate::percent::test::test_of;
    use currency::test::{Nls, Usdc};

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
    fn checked_add() {
        let amount1 = 10;
        let amount2 = 20;

        assert_eq!(
            Some(usdc(amount1 + amount2)),
            usdc(amount1).checked_add(usdc(amount2))
        );

        assert_eq!(
            Some(usdc(Amount::MAX)),
            usdc(Amount::MAX).checked_add(usdc(Amount::default()))
        );

        assert_eq!(
            Some(usdc(Amount::MAX)),
            usdc(Amount::MAX - amount2).checked_add(usdc(amount2))
        );

        assert_eq!(
            None,
            usdc(Amount::MAX - amount2).checked_add(usdc(amount2 + 1))
        );
    }

    #[test]
    fn saturating_sub() {
        assert_eq!(usdc(17), usdc(21).saturating_sub(usdc(4)));

        assert_eq!(usdc(1), usdc(21).saturating_sub(usdc(20)));

        assert_eq!(usdc(0), usdc(21).saturating_sub(usdc(21)));

        assert_eq!(usdc(0), usdc(21).saturating_sub(usdc(22)));

        assert_eq!(usdc(0), usdc(21).saturating_sub(usdc(122)));
    }

    #[test]
    #[should_panic = "overflow with real data"]
    fn add_panic() {
        let _ = usdc(Amount::MAX) + usdc(1);
    }

    #[test]
    fn checked_mul() {
        let amount1 = 10;
        let amount2 = 20;

        assert_eq!(
            Some(usdc(amount1 * amount2)),
            usdc(amount1).checked_mul(amount2)
        );

        assert_eq!(Some(usdc(Amount::MAX)), usdc(Amount::MAX).checked_mul(1));

        assert_eq!(
            Some(usdc(Amount::MAX)),
            usdc(Amount::MAX / 5).checked_mul(5)
        );

        assert_eq!(None, usdc(Amount::MAX / 5).checked_mul(5 + 1));
    }

    #[test]
    #[should_panic]
    fn of_overflow() {
        let max_amount = usdc(Amount::MAX);
        test_of(1001, max_amount, max_amount);
    }

    #[test]
    fn checked_div() {
        assert_eq!(Some(usdc(18 / 3)), usdc(18).checked_div(3));
        assert_eq!(Some(usdc(0)), usdc(0).checked_div(5));
        assert_eq!(Some(usdc(17 / 3)), usdc(17).checked_div(3));
    }

    #[test]
    fn div_ceil() {
        assert_eq!(Some(usdc(17 / 3)), usdc(17).checked_div(3));
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
