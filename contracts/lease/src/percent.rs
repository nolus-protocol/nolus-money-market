use std::ops::{Add, Sub};

#[derive(Copy, Clone, Default, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Percent {
    val: u8,
}

pub const HUNDRED: Percent = Percent { val: 100u8 };

impl Percent {
    pub fn u8(&self) -> u8 {
        self.val
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
    use crate::percent::{Percent, HUNDRED};

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
        assert_eq!(from(101), HUNDRED + from(1));
        assert_eq!(from(1) + HUNDRED, HUNDRED + from(1));
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
        assert_eq!(from(90), HUNDRED - from(10));
        assert_eq!(from(0), from(u8::MAX) - from(u8::MAX));
    }

    #[test]
    #[should_panic]
    fn sub_overflow() {
        let _ = from(34) - from(35);
    }
}
