#[cfg(feature = "testing")]
use std::num::NonZeroU128;
use std::{
    any,
    cmp::Ordering,
    fmt::{Debug, Display, Formatter},
    iter::Sum,
    marker::PhantomData,
    ops::{Add, AddAssign, Sub, SubAssign},
};

use ::serde::{Deserialize, Serialize};

use currency::{Currency, CurrencyDef, Group, MemberOf};

use crate::zero::Zero;

pub use self::{
    dto::{CoinDTO, IntoDTO},
    external::Coin as ExternalCoinDTO,
};

mod amount;
mod amount_serde;
mod dto;
mod external;
mod fraction;
mod fractionable;
mod serde;

pub type Amount = u128;
#[cfg(feature = "testing")]
pub type NonZeroAmount = NonZeroU128;

#[derive(Serialize, Deserialize)]
pub struct Coin<C> {
    #[serde(with = "amount_serde")]
    amount: Amount,
    #[serde(skip)]
    currency: PhantomData<C>,
}

impl<C> Coin<C> {
    pub const fn new(amount: Amount) -> Self {
        Self {
            amount,
            currency: PhantomData,
        }
    }

    const fn may_new(may_amount: Option<Amount>) -> Option<Self> {
        if let Some(amount) = may_amount {
            Some(Self::new(amount))
        } else {
            None
        }
    }

    pub const fn is_zero(&self) -> bool {
        self.amount == Zero::ZERO
    }

    #[track_caller]
    pub const fn checked_add(self, rhs: Self) -> Option<Self> {
        Self::may_new(self.amount.checked_add(rhs.amount))
    }

    #[track_caller]
    pub fn saturating_sub(self, rhs: Self) -> Self {
        Coin::new(self.amount.saturating_sub(rhs.amount))
    }

    #[track_caller]
    pub const fn checked_sub(self, rhs: Self) -> Option<Self> {
        Self::may_new(self.amount.checked_sub(rhs.amount))
    }

    #[track_caller]
    pub const fn checked_mul(self, rhs: Amount) -> Option<Self> {
        Self::may_new(self.amount.checked_mul(rhs))
    }

    #[track_caller]
    pub const fn checked_div(self, rhs: Amount) -> Option<Self> {
        Self::may_new(self.amount.checked_div(rhs))
    }
}

impl<C> Coin<C>
where
    C: 'static,
{
    pub fn coerce_into<SameC>(self) -> Coin<SameC>
    where
        SameC: 'static,
    {
        debug_assert!(currency::equal::<C, SameC>());
        Coin::new(self.amount)
    }
}

impl<C> Clone for Coin<C> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<C> Copy for Coin<C> {}

impl<C> Debug for Coin<C> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Coin")
            .field("amount", &self.amount)
            .field("ticker", &self.currency)
            .finish()
    }
}

impl<C> Default for Coin<C> {
    fn default() -> Self {
        Self {
            amount: Default::default(),
            currency: Default::default(),
        }
    }
}

impl<C> Eq for Coin<C> {}

impl<C> PartialEq for Coin<C> {
    fn eq(&self, other: &Self) -> bool {
        self.amount.eq(&other.amount)
    }
}

impl<C> PartialOrd for Coin<C> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<C> Ord for Coin<C>
where
    Self: PartialOrd,
{
    fn cmp(&self, other: &Self) -> Ordering {
        self.amount.cmp(&other.amount)
    }
}

impl<C> Add for Coin<C> {
    type Output = Self;

    #[track_caller]
    fn add(self, rhs: Coin<C>) -> Self::Output {
        self.checked_add(rhs)
            .expect("addition should not overflow with real data")
    }
}

impl<C> Sub for Coin<C> {
    type Output = Self;

    #[track_caller]
    fn sub(mut self, rhs: Coin<C>) -> Self::Output {
        self -= rhs;
        self
    }
}

impl<C> AddAssign for Coin<C> {
    #[track_caller]
    fn add_assign(&mut self, rhs: Coin<C>) {
        self.amount += rhs.amount;
    }
}

impl<C> SubAssign for Coin<C> {
    #[track_caller]
    fn sub_assign(&mut self, rhs: Coin<C>) {
        self.amount -= rhs.amount;
    }
}

impl<C> Display for Coin<C> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{} {}", self.amount, any::type_name::<C>()))
    }
}

// TODO remove it when finish refactoring Fractionable
impl<C> From<Coin<C>> for Amount {
    fn from(coin: Coin<C>) -> Self {
        coin.amount
    }
}

pub trait WithCoin<VisitedG>
where
    VisitedG: Group,
{
    type Outcome;

    fn on<C>(self, coin: Coin<C>) -> Self::Outcome
    where
        C: CurrencyDef,
        C::Group: MemberOf<VisitedG> + MemberOf<VisitedG::TopG>;
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
    use std::any;

    use currency::test::{SuperGroupTestC1, SuperGroupTestC2};

    use crate::{fraction::Coprime, percent::test, test::coin};

    use super::{Amount, Coin};

    #[test]
    fn display() {
        assert_eq!(
            format!("25 {}", any::type_name::<SuperGroupTestC2>()),
            coin::coin2(25).to_string()
        );
        assert_eq!(
            format!("0 {}", any::type_name::<SuperGroupTestC1>()),
            coin::coin1(0).to_string()
        );
    }

    #[test]
    fn of_are() {
        test::test_of(10, coin::coin1(100), coin::coin1(1));
        test::test_of(11, coin::coin1(100), coin::coin1(1));
        test::test_of(11, coin::coin1(90), coin::coin1(0));
        test::test_of(11, coin::coin1(91), coin::coin1(1));
        test::test_of(110, coin::coin1(100), coin::coin1(11));
        test::test_of(12, coin::coin1(100), coin::coin1(1));
        test::test_of(12, coin::coin1(84), coin::coin1(1));
        test::test_of(12, coin::coin1(83), coin::coin1(0));
        test::test_of(18, coin::coin1(100), coin::coin1(1));
        test::test_of(18, coin::coin1(56), coin::coin1(1));
        test::test_of(18, coin::coin1(55), coin::coin1(0));
        test::test_of(18, coin::coin1(120), coin::coin1(2));
        test::test_of(18, coin::coin1(112), coin::coin1(2));
        test::test_of(18, coin::coin1(111), coin::coin1(1));
        test::test_of(1000, coin::coin1(Amount::MAX), coin::coin1(Amount::MAX));
    }

    #[test]
    fn is_zero() {
        assert!(coin::coin1(0).is_zero());
        assert!(!coin::coin1(1).is_zero());
    }

    #[test]
    fn checked_add() {
        let amount1 = 10;
        let amount2 = 20;

        assert_eq!(
            Some(coin::coin1(amount1 + amount2)),
            coin::coin1(amount1).checked_add(coin::coin1(amount2))
        );

        assert_eq!(
            Some(coin::coin1(Amount::MAX)),
            coin::coin1(Amount::MAX).checked_add(coin::coin1(Amount::default()))
        );

        assert_eq!(
            Some(coin::coin1(Amount::MAX)),
            coin::coin1(Amount::MAX - amount2).checked_add(coin::coin1(amount2))
        );

        assert_eq!(
            None,
            coin::coin1(Amount::MAX - amount2).checked_add(coin::coin1(amount2 + 1))
        );
    }

    #[test]
    fn saturating_sub() {
        assert_eq!(
            coin::coin1(17),
            coin::coin1(21).saturating_sub(coin::coin1(4))
        );

        assert_eq!(
            coin::coin1(1),
            coin::coin1(21).saturating_sub(coin::coin1(20))
        );

        assert_eq!(
            coin::coin1(0),
            coin::coin1(21).saturating_sub(coin::coin1(21))
        );

        assert_eq!(
            coin::coin1(0),
            coin::coin1(21).saturating_sub(coin::coin1(22))
        );

        assert_eq!(
            coin::coin1(0),
            coin::coin1(21).saturating_sub(coin::coin1(122))
        );
    }

    #[test]
    fn checked_sub() {
        assert_eq!(
            Some(coin::coin1(17)),
            coin::coin1(21).checked_sub(coin::coin1(4))
        );

        assert_eq!(
            Some(coin::coin1(1)),
            coin::coin1(21).checked_sub(coin::coin1(20))
        );

        assert_eq!(
            Some(coin::coin1(0)),
            coin::coin1(21).checked_sub(coin::coin1(21))
        );

        assert_eq!(None, coin::coin1(21).checked_sub(coin::coin1(22)));

        assert_eq!(None, coin::coin1(21).checked_sub(coin::coin1(122)));
    }

    #[test]
    #[should_panic = "overflow with real data"]
    fn add_panic() {
        let _ = coin::coin1(Amount::MAX) + coin::coin1(1);
    }

    #[test]
    fn checked_mul() {
        let amount1 = 10;
        let amount2 = 20;

        assert_eq!(
            Some(coin::coin1(amount1 * amount2)),
            coin::coin1(amount1).checked_mul(amount2)
        );

        assert_eq!(
            Some(coin::coin1(Amount::MAX)),
            coin::coin1(Amount::MAX).checked_mul(1)
        );

        assert_eq!(
            Some(coin::coin1(Amount::MAX)),
            coin::coin1(Amount::MAX / 5).checked_mul(5)
        );

        assert_eq!(None, coin::coin1(Amount::MAX / 5).checked_mul(5 + 1));
    }

    #[test]
    #[should_panic]
    fn of_overflow() {
        let max_amount = coin::coin1(Amount::MAX);
        test::test_of(1001, max_amount, max_amount);
    }

    #[test]
    fn checked_div() {
        assert_eq!(Some(coin::coin1(18 / 3)), coin::coin1(18).checked_div(3));
        assert_eq!(Some(coin::coin1(0)), coin::coin1(0).checked_div(5));
        assert_eq!(Some(coin::coin1(17 / 3)), coin::coin1(17).checked_div(3));
    }

    #[test]
    fn div_ceil() {
        assert_eq!(Some(coin::coin1(17 / 3)), coin::coin1(17).checked_div(3));
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
        let coins = vec![
            coin::coin1(1),
            coin::coin1(2),
            coin::coin1(3),
            coin::coin1(4),
            coin::coin1(5),
        ];
        let exp_sum = coin::coin1(15);
        assert_eq!(coins.iter().sum::<Coin<SuperGroupTestC1>>(), exp_sum);
        assert_eq!(coins.into_iter().sum::<Coin<SuperGroupTestC1>>(), exp_sum);
    }

    #[test]
    fn coerce() {
        assert_eq!(coin::coin1(100), coin::coin1(100).coerce_into());
    }

    fn coprime_impl(gcd: Amount, a1: Amount, a2: Amount) {
        assert_eq!(
            (coin::coin1(a1 / gcd), coin::coin2(a2 / gcd)),
            coin::coin1(a1).to_coprime_with(coin::coin2(a2))
        );
        assert_eq!(
            (coin::coin2(a1 / gcd), coin::coin2(a2 / gcd)),
            coin::coin2(a1).to_coprime_with(coin::coin2(a2))
        );
    }
}
