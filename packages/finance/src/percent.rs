use std::{
    fmt::{Debug, Display, Formatter, Result},
    ops::{Add, Div, Mul, Sub},
};

use cosmwasm_std::{Coin, OverflowError, OverflowOperation, Uint256};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::error::Result as FinanceResult;

pub trait Percentable: Mul<Percent, Output = <Self as Percentable>::Intermediate> {
    type Intermediate: Div<Percent, Output = <Self as Percentable>::Result>;
    type Result: Percentable;
}

pub struct Coin256 {
    pub denom: String,
    pub amount: Uint256,
}
impl Mul<Percent> for &Coin {
    type Output = Coin256;

    fn mul(self, rhs: Percent) -> Self::Output {
        self.clone().mul(rhs)
    }
}

impl Mul<Percent> for Coin {
    type Output = Coin256;

    fn mul(self, rhs: Percent) -> Self::Output {
        Self::Output {
            denom: self.denom,
            amount: Uint256::from(self.amount).mul(Uint256::from(rhs.0)),
        }
    }
}

impl Div<Percent> for Coin256 {
    type Output = Coin;

    fn div(self, rhs: Percent) -> Self::Output {
        let amount256 = self.amount.div(Uint256::from(rhs.0));
        Self::Output {
            denom: self.denom,
            amount: amount256.try_into().expect("Overflow computing percent"),
        }
    }
}

impl Percentable for Coin {
    type Intermediate = Coin256;
    type Result = Self;
}

impl Percentable for &Coin {
    type Intermediate = Coin256;
    type Result = Coin;
}

#[derive(
    Copy, Clone, Default, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
#[serde(transparent)]
pub struct Percent(u32); //value in permille

impl Percent {
    pub const ZERO: Self = Self::from_permille(0);
    pub const HUNDRED: Self = Self::from_permille(1000);
    const PERMILLE_TO_PERCENT_RATIO: u32 = 10;

    pub fn from_percent(percent: u16) -> Self {
        Self(u32::from(percent) * Self::PERMILLE_TO_PERCENT_RATIO)
    }

    pub const fn from_permille(permille: u32) -> Self {
        Self(permille)
    }

    pub fn of<P>(&self, percentable: P) -> <P as Percentable>::Result
    where
        P: Percentable,
    {
        percentable * *self / Percent::HUNDRED
    }

    /// the inverse of `Percent::of`
    /// If %.of(X) -> Y, then %.are(Y) -> X
    /// :pre self != 0
    pub fn are(&self, amount: &Coin) -> Coin {
        debug_assert!(self != &Self::ZERO);
        let new_quantity = amount.amount.multiply_ratio(Percent::HUNDRED.0, self.0);
        Coin {
            amount: new_quantity,
            denom: amount.denom.clone(),
        }
    }

    pub fn checked_add(self, other: Self) -> FinanceResult<Self> {
        self.0
            .checked_add(other.0)
            .map(Self::from_permille)
            .ok_or_else(|| OverflowError::new(OverflowOperation::Add, self, other).into())
    }

    pub fn checked_sub(self, other: Self) -> FinanceResult<Self> {
        self.0
            .checked_sub(other.0)
            .map(Self::from_permille)
            .ok_or_else(|| OverflowError::new(OverflowOperation::Sub, self, other).into())
    }
}

impl Display for Percent {
    fn fmt(&self, f: &mut Formatter) -> Result {
        write!(f, "{:.1}%", self.0 / Self::PERMILLE_TO_PERCENT_RATIO)
    }
}

impl Add<Percent> for Percent {
    type Output = Self;

    fn add(self, rhs: Self) -> Self {
        Self(
            self.0
                .checked_add(rhs.0)
                .expect("attempt to add with overflow"),
        )
    }
}

impl<'a> Add<&'a Percent> for Percent {
    type Output = Self;

    fn add(self, rhs: &'a Percent) -> Self {
        self + *rhs
    }
}

impl Sub<Percent> for Percent {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self {
        Self(
            self.0
                .checked_sub(rhs.0)
                .expect("attempt to subtract with overflow"),
        )
    }
}

impl<'a> Sub<&'a Percent> for Percent {
    type Output = Self;

    fn sub(self, rhs: &'a Percent) -> Self {
        self - *rhs
    }
}

#[cfg(test)]
mod test {
    use cosmwasm_std::Coin;

    use crate::percent::Percent;

    fn from(permille: u32) -> Percent {
        Percent::from_permille(permille)
    }

    #[test]
    fn from_percent() {
        assert_eq!(Percent(0), Percent::from_percent(0));
        assert_eq!(Percent(100), Percent::from_percent(10));
    }

    #[test]
    fn from_permille() {
        assert_eq!(Percent(0), Percent::from_permille(0));
        assert_eq!(Percent(10), Percent::from_permille(10));
    }

    #[test]
    fn test_zero() {
        let d = String::from("sfw");
        assert_eq!(Coin::new(0, d.clone()), Percent::ZERO.of(&Coin::new(10, d)))
    }

    #[test]
    fn test_hundred() {
        let d = String::from("sfw");
        let amount = 123;
        assert_eq!(
            Coin::new(amount, d.clone()),
            Percent::HUNDRED.of(&Coin::new(amount, d))
        )
    }

    #[test]
    fn add() {
        assert_eq!(from(40), from(25) + from(15));
        assert_eq!(from(39), from(0) + from(39));
        assert_eq!(from(39), from(39) + from(0));
        assert_eq!(from(1001), Percent::HUNDRED + from(1));
        assert_eq!(from(1) + Percent::HUNDRED, Percent::HUNDRED + from(1));
        assert_eq!(from(u32::MAX), from(u32::MAX) + from(0));
    }

    #[test]
    #[should_panic]
    fn add_overflow() {
        let _ = from(u32::MAX) + from(1);
    }

    #[test]
    fn sub() {
        assert_eq!(from(67), from(79) - from(12));
        assert_eq!(from(0), from(34) - from(34));
        assert_eq!(from(39), from(39) - from(0));
        assert_eq!(from(990), Percent::HUNDRED - from(10));
        assert_eq!(from(0), from(u32::MAX) - from(u32::MAX));
    }

    #[test]
    #[should_panic]
    fn sub_overflow() {
        let _ = from(34) - from(35);
    }

    fn test_of_are(permille: u32, quantity: u128, exp: u128) {
        let d = String::from("ABC");
        let of = Coin::new(quantity, d.clone());
        let exp = Coin::new(exp, d);
        assert_eq!(exp, Percent::from_permille(permille).of(&of));
        if permille != 0 {
            assert_eq!(of, Percent::from_permille(permille).are(&exp));
        }
    }

    #[test]
    fn of_are() {
        test_of_are(100, 50, 5);
        test_of_are(100, 5000, 500);
        test_of_are(101, 5000, 505);
        test_of_are(200, 50, 10);
        test_of_are(0, 120, 0);
        test_of_are(1, 1000, 1);
        test_of_are(1, 0, 0);
        test_of_are(200, 0, 0);
        test_of_are(1200, 50, 60);
        test_of_are(12, 500, 6);
        test_of_are(1000, u128::MAX, u128::MAX);
    }

    #[test]
    #[should_panic]
    fn of_overflow() {
        test_of_are(1001, u128::MAX, u128::MAX);
    }

    #[test]
    #[should_panic]
    fn are_overflow() {
        test_of_are(999, u128::MAX, u128::MAX);
    }
}
