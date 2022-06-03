use cosmwasm_std::Fraction;

use crate::{percent::Units, percentable::Percentable};

type Double64 = u128;

impl Percentable for u64 {
    fn safe_mul<F>(self, fraction: &F) -> Self
    where
        F: Fraction<Units>,
    {
        let res_double: Double64 = Double64::from(self) * Double64::from(fraction.numerator())
            / Double64::from(fraction.denominator());
        res_double.try_into().expect("unexpected overflow")
    }
}

#[cfg(test)]
mod test {
    use crate::percent::test::{test_of_are, test_of, test_are};

    #[test]
    fn of_are() {
        test_of_are(100, 50, 5);
        test_of_are(100, 5000, 500);
        test_of_are(101, 5000, 505);
        test_of_are(200, 50, 10);
        test_of(0, 120, 0);
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
        test_of(2001, u64::MAX / 2, u64::MAX);
    }

    #[test]
    #[should_panic]
    fn are_overflow() {
        test_are(999, u64::MAX, u64::MAX);
    }

    #[test]
    #[should_panic]
    fn are_div_zero() {
        test_are(0, u64::MAX, u64::MAX);
    }
}
