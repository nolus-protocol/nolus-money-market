use std::{
    fmt::{Display, Formatter, Result},
    ops::{Add, Sub},
};

use cosmwasm_std::{Coin, OverflowError, OverflowOperation};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::error::ContractResult;

#[derive(
    Copy, Clone, Default, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
#[serde(transparent)]
pub struct Percent {
    val: u8,
}

impl Percent {
    pub const ZERO: Percent = Percent { val: 0u8 };
    pub const HUNDRED: Percent = Percent { val: 100u8 };

    pub fn u8(&self) -> u8 {
        self.val
    }

    pub fn of(&self, amount: &Coin) -> Coin {
        let new_quantity = amount
            .amount
            .multiply_ratio(self.val, Percent::HUNDRED.u8());
        Coin {
            amount: new_quantity,
            denom: amount.denom.clone(),
        }
    }

    /// the inverse of `Percent::of`
    /// If %.of(X) -> Y, then %.are(Y) -> X
    /// :pre self != 0
    pub fn are(&self, amount: &Coin) -> Coin {
        debug_assert!(self.val != 0);
        let new_quantity = amount
            .amount
            .multiply_ratio(Percent::HUNDRED.u8(), self.val);
        Coin {
            amount: new_quantity,
            denom: amount.denom.clone(),
        }
    }

    pub fn checked_add(self, other: Self) -> ContractResult<Self> {
        self.val
            .checked_add(other.val)
            .map(Self::from)
            .ok_or_else(|| OverflowError::new(OverflowOperation::Add, self, other).into())
    }

    pub fn checked_sub(self, other: Self) -> ContractResult<Self> {
        self.val
            .checked_sub(other.val)
            .map(Self::from)
            .ok_or_else(|| OverflowError::new(OverflowOperation::Sub, self, other).into())
    }
}

impl From<u8> for Percent {
    fn from(val: u8) -> Self {
        Self { val }
    }
}

impl From<Percent> for u8 {
    fn from(p: Percent) -> Self {
        p.val
    }
}

impl Display for Percent {
    fn fmt(&self, f: &mut Formatter) -> Result {
        self.val.fmt(f)
    }
}

impl Add<Percent> for Percent {
    type Output = Self;

    fn add(self, rhs: Self) -> Self {
        Self {
            val: self
                .u8()
                .checked_add(rhs.u8())
                .expect("attempt to add with overflow"),
        }
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
        Self {
            val: self
                .u8()
                .checked_sub(rhs.u8())
                .expect("attempt to subtract with overflow"),
        }
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

    fn from(val: u8) -> Percent {
        val.into()
    }

    #[test]
    fn from_u8() {
        let val = 10u8;
        assert_eq!(Percent { val: 10 }, Percent::from(val));
        assert_eq!(val, Percent::from(val).u8());
    }

    #[test]
    fn into_u8() {
        let val = 35u8;
        let p = Percent { val };
        let val_result: u8 = p.into();
        assert_eq!(val, val_result);
    }

    #[test]
    fn add() {
        assert_eq!(from(40), from(25) + from(15));
        assert_eq!(from(39), from(0) + from(39));
        assert_eq!(from(39), from(39) + from(0));
        assert_eq!(from(101), Percent::HUNDRED + from(1));
        assert_eq!(from(1) + Percent::HUNDRED, Percent::HUNDRED + from(1));
        assert_eq!(from(u8::MAX), from(u8::MAX) + from(0));
    }

    #[test]
    #[should_panic]
    fn add_overflow() {
        let _ = from(u8::MAX) + from(1);
    }

    #[test]
    fn sub() {
        assert_eq!(from(67), from(79) - from(12));
        assert_eq!(from(0), from(34) - from(34));
        assert_eq!(from(39), from(39) - from(0));
        assert_eq!(from(90), Percent::HUNDRED - from(10));
        assert_eq!(from(0), from(u8::MAX) - from(u8::MAX));
    }

    #[test]
    #[should_panic]
    fn sub_overflow() {
        let _ = from(34) - from(35);
    }

    fn test_of(percent: u8, quantity: u128, exp: u128) {
        let d = String::from("ABC");
        assert_eq!(
            Coin::new(exp, d.clone()),
            Percent::from(percent).of(&Coin::new(quantity, d))
        );
    }

    #[test]
    fn of() {
        test_of(10, 50, 5);
        test_of(20, 50, 10);
        test_of(0, 120, 0);
        test_of(20, 0, 0);
        test_of(120, 50, 60);
        test_of(100, u128::MAX, u128::MAX);
    }

    #[test]
    #[should_panic]
    fn of_overflow() {
        test_of(101, u128::MAX, u128::MAX);
    }

    fn test_are(percent: u8, quantity: u128, exp: u128) {
        debug_assert!(percent != 0);
        let d = String::from("ABC");
        assert_eq!(
            Coin::new(exp, d.clone()),
            Percent::from(percent).are(&Coin::new(quantity, d))
        );
    }

    #[test]
    fn are() {
        test_are(10, 5, 50);
        test_are(20, 10, 50);
        test_are(1, 1, 100);
        test_are(20, 0, 0);
        test_are(120, 60, 50);
        test_are(100, u128::MAX, u128::MAX);
    }

    #[test]
    #[should_panic]
    fn are_overflow() {
        test_are(99, u128::MAX, u128::MAX);
    }
}
