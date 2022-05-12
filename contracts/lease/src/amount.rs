use std::ops::{Add, Sub};

use cosmwasm_std::Uint128;

use crate::percent::{Percent, HUNDRED};

#[derive(Copy, Clone, Default, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Amount {
    val: Uint128,
}

impl Amount {
    pub fn percent(&self, percent: Percent) -> Amount {
        Amount {
            val: self.val.multiply_ratio(percent.u8(), HUNDRED.u8()),
        }
    }
}

impl From<Uint128> for Amount {
    fn from(val: Uint128) -> Self {
        Self { val }
    }
}

impl From<Amount> for Uint128 {
    fn from(amount: Amount) -> Self {
        amount.val
    }
}

impl From<u128> for Amount {
    fn from(val: u128) -> Self {
        Self { val: val.into() }
    }
}

impl From<Amount> for u128 {
    fn from(amount: Amount) -> Self {
        amount.val.into()
    }
}

impl Add<Amount> for Amount {
    type Output = Self;

    fn add(self, rhs: Self) -> Self {
        Self {
            val: self
                .val
                .checked_add(rhs.val)
                .expect("attempt to add with overflow"),
        }
    }
}

impl<'a> Add<&'a Amount> for Amount {
    type Output = Self;

    fn add(self, rhs: &'a Self) -> Self {
        self + *rhs
    }
}

impl Sub<Amount> for Amount {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self {
        Self {
            val: self
                .val
                .checked_sub(rhs.val)
                .expect("attempt to subtract with overflow"),
        }
    }
}

impl<'a> Sub<&'a Amount> for Amount {
    type Output = Self;

    fn sub(self, rhs: &'a Amount) -> Self {
        self - *rhs
    }
}

#[cfg(test)]
mod test {
    use cosmwasm_std::Uint128;

    use crate::{amount::Amount, percent::HUNDRED};

    fn from(val: u128) -> Amount {
        val.into()
    }

    #[test]
    fn percents() {
        let a = Amount::from(5);
        assert_eq!(a, a.percent(HUNDRED));
        assert_eq!(from(1), a.percent(20.into()));
        assert_eq!(from(0), a.percent(19.into()));
        assert_eq!(from(0), a.percent(0.into()));
        assert_eq!(from(6), a.percent(120.into()));
        assert_eq!(from(10), a.percent(200.into()));
    }

    #[test]
    fn from_into_u128() {
        assert_eq!(10u128, Amount::from(10).into());
        let a = from(100);
        assert_eq!(a, Amount::from(u128::from(a)));
    }

    #[test]
    fn from_into_uint128() {
        assert_eq!(Uint128::new(10), Amount::from(10).into());
        let a = from(100);
        assert_eq!(a, Amount::from(Uint128::from(a)));
    }

    #[test]
    fn add() {
        assert_eq!(from(400), from(250) + from(150));
        assert_eq!(from(39), from(0) + from(39));
        assert_eq!(from(39), from(39) + from(0));
        assert_eq!(from(u128::MAX), from(u128::MAX) + from(0));
    }

    #[test]
    #[should_panic]
    fn add_overflow() {
        let _ = from(u128::MAX) + from(1);
    }

    #[test]
    fn sub() {
        assert_eq!(from(67), from(79) - from(12));
        assert_eq!(from(0), from(34) - from(34));
        assert_eq!(from(39), from(39) - from(0));
        assert_eq!(from(0), from(u128::MAX) - from(u128::MAX));
    }

    #[test]
    #[should_panic]
    fn sub_overflow() {
        let _ = from(34) - from(35);
    }
}
