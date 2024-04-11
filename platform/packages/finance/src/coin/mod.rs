#[cfg(feature = "testing")]
use std::num::NonZeroU128;
use std::{
    cmp::Ordering,
    fmt::{Debug, Display, Formatter},
    iter::Sum,
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
#[cfg(feature = "testing")]
pub type NonZeroAmount = NonZeroU128;

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct Coin<C>
where
    C: ?Sized,
{
    amount: Amount,
    #[serde(skip)]
    ticker: PhantomData<C>,
}

impl<C> Clone for Coin<C>
where
    C: ?Sized,
{
    fn clone(&self) -> Self {
        *self
    }
}

impl<C> Copy for Coin<C> where C: ?Sized {}

impl<C> Coin<C>
where
    C: ?Sized,
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
        OtherC: ?Sized,
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

impl<C> Debug for Coin<C>
where
    C: ?Sized,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Coin")
            .field("amount", &self.amount)
            .field("ticker", &self.ticker)
            .finish()
    }
}

impl<C> Default for Coin<C>
where
    C: ?Sized,
{
    fn default() -> Self {
        Self {
            amount: Default::default(),
            ticker: Default::default(),
        }
    }
}

impl<C> PartialEq for Coin<C>
where
    C: ?Sized,
{
    fn eq(&self, other: &Self) -> bool {
        self.amount == other.amount
    }
}

impl<C> Eq for Coin<C> where C: ?Sized {}

impl<C> PartialOrd for Coin<C>
where
    C: ?Sized,
{
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<C> Ord for Coin<C>
where
    C: ?Sized,
    Self: PartialOrd,
{
    fn cmp(&self, other: &Self) -> Ordering {
        self.amount.cmp(&other.amount)
    }
}

impl<C> Zero for Coin<C>
where
    C: ?Sized,
{
    const ZERO: Self = Self::new(Zero::ZERO);
}

impl<C> Add<Coin<C>> for Coin<C>
where
    C: ?Sized,
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
    C: ?Sized,
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
    C: ?Sized,
{
    #[track_caller]
    fn add_assign(&mut self, rhs: Coin<C>) {
        self.amount += rhs.amount;
    }
}

impl<C> SubAssign<Coin<C>> for Coin<C>
where
    C: ?Sized,
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
    C: ?Sized,
{
    fn from(amount: Amount) -> Self {
        Self::new(amount)
    }
}

impl<C> From<Coin<C>> for Amount
where
    C: ?Sized,
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

impl<CoinCRef, C> Sum<CoinCRef> for Coin<C>
where
    CoinCRef: AsRef<Coin<C>>,
    C: Currency,
{
    fn sum<I: Iterator<Item = CoinCRef>>(coins: I) -> Self {
        coins.fold(Self::default(), |acc, next_coin| acc + *next_coin.as_ref())
    }
}

impl<C> AsRef<Self> for Coin<C> {
    fn as_ref(&self) -> &Self {
        self
    }
}

#[cfg(test)]
mod test {
    use currency::{
        test::{SuperGroupTestC1, SuperGroupTestC2},
        Currency,
    };

    use crate::percent::test::test_of;

    use super::{Amount, Coin};

    #[test]
    fn display() {
        assert_eq!(
            format!("25 {}", SuperGroupTestC2::TICKER),
            coin2(25).to_string()
        );
        assert_eq!(
            format!("0 {}", SuperGroupTestC1::TICKER),
            coin1(0).to_string()
        );
    }

    #[test]
    fn of_are() {
        test_of(10, coin1(100), coin1(1));
        test_of(11, coin1(100), coin1(1));
        test_of(11, coin1(90), coin1(0));
        test_of(11, coin1(91), coin1(1));
        test_of(110, coin1(100), coin1(11));
        test_of(12, coin1(100), coin1(1));
        test_of(12, coin1(84), coin1(1));
        test_of(12, coin1(83), coin1(0));
        test_of(18, coin1(100), coin1(1));
        test_of(18, coin1(56), coin1(1));
        test_of(18, coin1(55), coin1(0));
        test_of(18, coin1(120), coin1(2));
        test_of(18, coin1(112), coin1(2));
        test_of(18, coin1(111), coin1(1));
        test_of(1000, coin1(Amount::MAX), coin1(Amount::MAX));
    }

    #[test]
    fn is_zero() {
        assert!(coin1(0).is_zero());
        assert!(!coin1(1).is_zero());
    }

    #[test]
    fn checked_add() {
        let amount1 = 10;
        let amount2 = 20;

        assert_eq!(
            Some(coin1(amount1 + amount2)),
            coin1(amount1).checked_add(coin1(amount2))
        );

        assert_eq!(
            Some(coin1(Amount::MAX)),
            coin1(Amount::MAX).checked_add(coin1(Amount::default()))
        );

        assert_eq!(
            Some(coin1(Amount::MAX)),
            coin1(Amount::MAX - amount2).checked_add(coin1(amount2))
        );

        assert_eq!(
            None,
            coin1(Amount::MAX - amount2).checked_add(coin1(amount2 + 1))
        );
    }

    #[test]
    fn saturating_sub() {
        assert_eq!(coin1(17), coin1(21).saturating_sub(coin1(4)));

        assert_eq!(coin1(1), coin1(21).saturating_sub(coin1(20)));

        assert_eq!(coin1(0), coin1(21).saturating_sub(coin1(21)));

        assert_eq!(coin1(0), coin1(21).saturating_sub(coin1(22)));

        assert_eq!(coin1(0), coin1(21).saturating_sub(coin1(122)));
    }

    #[test]
    #[should_panic = "overflow with real data"]
    fn add_panic() {
        let _ = coin1(Amount::MAX) + coin1(1);
    }

    #[test]
    fn checked_mul() {
        let amount1 = 10;
        let amount2 = 20;

        assert_eq!(
            Some(coin1(amount1 * amount2)),
            coin1(amount1).checked_mul(amount2)
        );

        assert_eq!(Some(coin1(Amount::MAX)), coin1(Amount::MAX).checked_mul(1));

        assert_eq!(
            Some(coin1(Amount::MAX)),
            coin1(Amount::MAX / 5).checked_mul(5)
        );

        assert_eq!(None, coin1(Amount::MAX / 5).checked_mul(5 + 1));
    }

    #[test]
    #[should_panic]
    fn of_overflow() {
        let max_amount = coin1(Amount::MAX);
        test_of(1001, max_amount, max_amount);
    }

    #[test]
    fn checked_div() {
        assert_eq!(Some(coin1(18 / 3)), coin1(18).checked_div(3));
        assert_eq!(Some(coin1(0)), coin1(0).checked_div(5));
        assert_eq!(Some(coin1(17 / 3)), coin1(17).checked_div(3));
    }

    #[test]
    fn div_ceil() {
        assert_eq!(Some(coin1(17 / 3)), coin1(17).checked_div(3));
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

    #[test]
    fn sum() {
        let coins = vec![coin1(1), coin1(2), coin1(3), coin1(4), coin1(5)];
        let exp_sum = coin1(15);
        assert_eq!(coins.iter().sum::<Coin<SuperGroupTestC1>>(), exp_sum);
        assert_eq!(coins.into_iter().sum::<Coin<SuperGroupTestC1>>(), exp_sum);
    }

    fn coprime_impl(gcd: Amount, a1: Amount, a2: Amount) {
        assert_eq!(
            (coin1(a1 / gcd), coin2(a2 / gcd)),
            coin1(a1).into_coprime_with(coin2(a2))
        );
        assert_eq!(
            (coin2(a1 / gcd), coin2(a2 / gcd)),
            coin2(a1).into_coprime_with(coin2(a2))
        );
    }

    fn coin1(amount: Amount) -> Coin<SuperGroupTestC1> {
        Coin::new(amount)
    }

    fn coin2(amount: Amount) -> Coin<SuperGroupTestC2> {
        Coin::new(amount)
    }
}
