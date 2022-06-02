use std::ops::{Div, Mul};

use crate::{percent::Percent, percentable::Percentable};

impl Percentable for u64 {
    type Intermediate = u128;
    type Result = Self;
}
impl Mul<Percent> for u64 {
    type Output = u128;

    fn mul(self, rhs: Percent) -> Self::Output {
        Self::Output::from(self).mul(Self::Output::from(rhs.units()))
    }
}
impl Div<Percent> for u128 {
    type Output = u64;

    fn div(self, rhs: Percent) -> Self::Output {
        let out128 = self.div(Self::from(rhs.units()));
        out128.try_into().expect("Overflow")
    }
}

#[cfg(test)]
mod test {
    use crate::percent::test::test_of_are;

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
        test_of_are(1000, u64::MAX, u64::MAX);
    }

    #[test]
    #[should_panic]
    fn of_overflow() {
        test_of_are(2001, u64::MAX / 2, u64::MAX);
    }

    #[test]
    #[should_panic]
    fn are_overflow() {
        test_of_are(999, u64::MAX, u64::MAX);
    }
}
