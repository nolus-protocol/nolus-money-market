use cosmwasm_std::Fraction;

use crate::{
    percent::{Percent, Units},
    percentable::Percentable,
};

use super::Integer;

impl Integer for Units {
    type DoubleInteger = u64;
}

impl Percentable for Percent {
    fn safe_mul<F>(self, fraction: &F) -> Self
    where
        F: Fraction<Units>,
    {
        Percent::from_permille(self.units().safe_mul(fraction))
    }
}

#[cfg(test)]
mod test {
    use crate::percent::{
        test::{test_are, test_of, test_of_are},
        Percent, Units,
    };

    #[test]
    fn of_are() {
        test_of_are(100, Percent::from_percent(40), Percent::from_percent(4));
        test_of_are(100, Percent::from_percent(40), Percent::from_permille(40));
        test_of_are(10, Percent::from_percent(800), Percent::from_percent(8));
        test_of_are(10, Percent::from_permille(8900), Percent::from_permille(89));
        test_of_are(1, Percent::from_percent(12300), Percent::from_permille(123));
        test_of(1, Percent::from_percent(12345), Percent::from_permille(123));
        test_are(1, Percent::from_permille(123), Percent::from_percent(12300));
        test_of(0, Percent::from_percent(123), Percent::from_percent(0));
        test_of_are(
            1000,
            Percent::from_permille(Units::MAX),
            Percent::from_permille(Units::MAX),
        );
        test_of_are(
            2000,
            Percent::from_permille(Units::MAX / 2),
            Percent::from_permille(Units::MAX - 1),
        );

        test_of_are(1000, Percent::HUNDRED, Percent::HUNDRED);
        test_of_are(100, Percent::ZERO, Percent::ZERO);
    }

    #[test]
    #[should_panic]
    fn of_overflow() {
        Percent::from_permille(1001).of(Percent::from_permille(Units::MAX));
    }

    #[test]
    #[should_panic]
    fn are_overflow() {
        Percent::from_permille(999).are(Percent::from_permille(Units::MAX));
    }

    #[test]
    #[should_panic]
    fn are_div_zero() {
        Percent::ZERO.are(Percent::from_permille(10));
    }
}
