use bound::BoundPercent;

use crate::{
    coin::{Amount, Coin},
    ratio::{CheckedAdd, CheckedMul},
};

pub mod bound;

pub type Units = u32;

pub const HUNDRED_BOUND: Units = 1000;
pub const MAX_BOUND: Units = Units::MAX;

pub type Percent100 = BoundPercent<HUNDRED_BOUND>;
pub type Percent = BoundPercent<MAX_BOUND>;

impl CheckedMul for Units {
    type Output = Self;

    fn checked_mul(self, rhs: Self) -> Option<Self::Output> {
        self.checked_mul(rhs)
    }
}

impl<C> CheckedMul<Coin<C>> for Units {
    type Output = Coin<C>;

    fn checked_mul(self, rhs: Coin<C>) -> Option<Self::Output> {
        rhs.checked_mul(self.into())
    }
}

impl CheckedMul<Amount> for Units {
    type Output = Amount;

    fn checked_mul(self, rhs: Amount) -> Option<Self::Output> {
        rhs.checked_mul(self.into())
    }
}

impl CheckedAdd for Units {
    type Output = Self;

    fn checked_add(self, rhs: Self) -> Option<Self::Output> {
        self.checked_add(rhs)
    }
}

#[cfg(test)]
pub(super) mod test {
    use currency::test::SubGroupTestC10;

    use crate::{
        coin::Coin,
        fraction::Fraction,
        percent::{Percent, Percent100},
        ratio::Rational,
    };

    use super::Units;

    #[test]
    fn from_percent() {
        assert_eq!(Percent100::from_percent(0), Percent100::new(0));
        assert_eq!(Percent100::from_percent(10), Percent100::new(100));
        assert_eq!(Percent100::from_percent(99), Percent100::new(990));
        assert_eq!(Percent100::from_percent(100), Percent100::new(1000));

        assert_eq!(Percent::from_percent(0), Percent::new(0));
        assert_eq!(Percent::from_percent(101), Percent::new(1010));
    }

    #[test]
    fn from_permille() {
        assert_eq!(Percent100::from_permille(0), Percent100::new(0));
        assert_eq!(Percent100::from_permille(10), Percent100::new(10));
        assert_eq!(Percent100::from_permille(1000), Percent100::new(1000));

        assert_eq!(Percent::from_permille(0), Percent::new(0));
        assert_eq!(Percent::from_permille(1001), Percent::new(1001));
        assert_eq!(Percent::from_permille(Units::MAX), Percent::new(Units::MAX));
    }

    #[test]
    fn test_zero() {
        let zero_amount = Coin::<SubGroupTestC10>::new(0);
        assert_eq!(
            zero_amount,
            Percent100::ZERO.of(Coin::<SubGroupTestC10>::new(10))
        );
        assert_eq!(
            zero_amount,
            Percent::ZERO.of(Coin::<SubGroupTestC10>::new(10)).unwrap()
        )
    }

    #[test]
    fn test_hundred() {
        let amount = Coin::<SubGroupTestC10>::new(123);
        assert_eq!(amount, Percent100::HUNDRED.of(amount));
        assert_eq!(amount, Percent::HUNDRED.of(amount).unwrap())
    }

    #[test]
    fn checked_add() {
        assert_eq!(from(40), from(25).checked_add(from(15)).unwrap());
        assert_eq!(from(39), from(0).checked_add(from(39)).unwrap());
        assert_eq!(from(39), from(39).checked_add(from(0)).unwrap());
        assert_eq!(Percent100::HUNDRED, from(999).checked_add(from(1)).unwrap());
    }

    #[test]
    fn add_overflow() {
        assert!(Percent100::HUNDRED.checked_add(from(1)).is_err());
        assert!(
            Percent::from_permille(Units::MAX)
                .checked_add(Percent::from_permille(1))
                .is_err()
        );
    }

    #[test]
    fn checked_sub() {
        assert_eq!(from(67), from(79).checked_sub(from(12)).unwrap());
        assert_eq!(from(0), from(34).checked_sub(from(34)).unwrap());
        assert_eq!(from(39), from(39).checked_sub(from(0)).unwrap());
        assert_eq!(
            from(990),
            Percent100::HUNDRED.checked_sub(from(10)).unwrap()
        );
        assert_eq!(
            from(0),
            Percent100::HUNDRED
                .checked_sub(from(Percent100::PERMILLE))
                .unwrap()
        );
    }

    #[test]
    fn sub_overflow() {
        assert!(from(34).checked_sub(from(35)).is_err())
    }

    #[test]
    fn display() {
        test_display("0%", 0);
        test_display("0.1%", 1);
        test_display("0.4%", 4);
        test_display("1%", 10);
        test_display("1.9%", 19);
        test_display("9%", 90);
        test_display("10.1%", 101);
        test_display("100%", 1000);
    }

    #[test]
    fn of() {
        test_of(
            100,
            Percent100::from_percent(40),
            Percent100::from_percent(4),
        );
        test_of(
            100,
            Percent100::from_percent(40),
            Percent100::from_permille(40),
        );
        test_of(
            10,
            Percent100::from_permille(800),
            Percent100::from_permille(8),
        );
        test_of(
            10,
            Percent100::from_permille(890),
            Percent100::from_permille(8),
        );
        test_of(1, Percent100::from_permille(123), Percent100::ZERO);
        test_of(0, Percent100::HUNDRED, Percent100::from_percent(0));
        test_of(
            1000,
            Percent100::from_permille(Percent100::PERMILLE),
            Percent100::from_permille(1000),
        );
        test_of(1000, Percent100::HUNDRED, Percent100::HUNDRED);
        test_of(100, Percent100::ZERO, Percent100::ZERO);
    }

    #[test]
    fn of_overflow() {
        assert!(Percent::from_permille(Units::MAX).of(Units::MAX).is_none())
    }

    #[test]
    fn rational_of_percents() {
        let v = 14u32;
        let r = Rational::new(Percent100::HUNDRED.units(), Percent100::HUNDRED.units());
        assert_eq!(v, r.checked_mul(v).unwrap());
    }

    #[test]
    fn rational_to_percents() {
        let n: Units = 189;
        let d: Units = 1890;
        let r = Rational::new(n, d);
        let res: Percent100 = r.checked_mul(Percent100::HUNDRED).unwrap();
        assert_eq!(Percent100::from_permille(n * 1000 / d), res);
    }

    fn test_of(permille: Units, quantity: Percent100, exp: Percent100) {
        let perm = Percent100::from_permille(permille);
        assert_eq!(
            exp,
            perm.of(quantity),
            "Calculating {} of {}",
            perm,
            quantity
        );
    }

    fn from(permille: Units) -> Percent100 {
        Percent100::from_permille(permille)
    }

    fn test_display(exp: &str, permilles: Units) {
        assert_eq!(exp, format!("{}", Percent100::from_permille(permilles)));
    }
}
