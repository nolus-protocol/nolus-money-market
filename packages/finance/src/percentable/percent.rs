use std::ops::{Div, Mul};

use crate::{
    percent::{Percent, Units},
    percentable::Percentable,
};

type DoubleUnit = u64;

impl Percentable for Percent {
    type Intermediate = DoubleUnit;
    type Result = Self;
}
impl Mul<Percent> for Percent {
    type Output = <Self as Percentable>::Intermediate;

    fn mul(self, rhs: Percent) -> Self::Output {
        debug_assert_eq!(Units::BITS * 2, Self::Output::BITS);
        Self::Output::from(self.units()).mul(Self::Output::from(rhs.units()))
    }
}
impl Div<Percent> for DoubleUnit {
    type Output = Percent;

    fn div(self, rhs: Percent) -> Self::Output {
        let out_double = self.div(Self::from(rhs.units()));
        let out: Units = out_double.try_into().expect("Overflow");
        Self::Output::from_permille(out)
    }
}

#[cfg(test)]
mod test {
    use crate::percent::{test::test_of_are, Percent, Units};

    #[test]
    fn of_are() {
        test_of_are(100, Percent::from_percent(40), Percent::from_percent(4));
        test_of_are(100, Percent::from_percent(40), Percent::from_permille(40));
        test_of_are(10, Percent::from_percent(800), Percent::from_percent(8));
        test_of_are(10, Percent::from_permille(8900), Percent::from_permille(89));
        test_of_are(1, Percent::from_percent(12300), Percent::from_permille(123));
        test_of_are(0, Percent::from_percent(123), Percent::from_percent(0));
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
}
