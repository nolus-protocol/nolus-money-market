use bound::BoundPercent;

use crate::{
    fraction::Fraction, fractionable::Fractionable, ratio::SimpleFraction, rational::Rational,
    traits::FractionUnit,
};

pub mod bound;

pub type Units = u32;

pub const HUNDRED_BOUND: Units = 1000;
pub const MAX_BOUND: Units = Units::MAX;

pub type Percent100 = BoundPercent<HUNDRED_BOUND>;
pub type Percent = BoundPercent<MAX_BOUND>;

impl FractionUnit for Units {}

impl Fraction<Percent100> for Percent100 {
    fn of<A>(self, whole: A) -> A
    where
        A: Fractionable<Percent100>,
    {
        let fraction: SimpleFraction<Percent100> = self.into();
        fraction
            .of(whole)
            .expect("TODO it won't be needed when I call ratio.of()")
    }
}

impl Rational<Percent> for Percent {
    fn of<A>(self, whole: A) -> Option<A>
    where
        A: Fractionable<Percent>,
    {
        let fraction: SimpleFraction<Percent> = self.into();
        fraction.of(whole)
    }
}

// impl From<Percent100> for Ratio<Percent100> {
//     fn from(percent: Percent100) -> Self {
//         Self::new(percent, Percent100::HUNDRED)
//     }
// }

// TODO remove this convertion after Ratio become a struct
impl From<Percent100> for SimpleFraction<Percent100> {
    fn from(percent: Percent100) -> Self {
        Self::new(percent, Percent100::HUNDRED)
    }
}

impl From<Percent> for SimpleFraction<Percent> {
    fn from(percent: Percent) -> Self {
        Self::new(percent, Percent::HUNDRED)
    }
}

#[cfg(test)]
pub(super) mod test {
    use std::fmt::{Debug, Display};

    use currency::test::SubGroupTestC10;

    use crate::{
        coin::Coin,
        fraction::Fraction,
        fractionable::Fractionable,
        percent::{Percent, Percent100},
        ratio::SimpleFraction,
        rational::Rational,
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
        assert!(
            Percent::from_permille(Units::MAX)
                .of(Percent::from_permille(Units::MAX))
                .is_none()
        )
    }

    #[test]
    fn rational_to_percents() {
        let n: Units = 189;
        let d: Units = 1890;
        let r = SimpleFraction::new(Percent::from_permille(n), Percent::from_permille(d));
        let res: Percent = r.of(Percent::HUNDRED).unwrap();
        assert_eq!(Percent::from_permille(n * 1000 / d), res);
    }

    // pub fn test_of(permille: Units, quantity: Percent100, exp: Percent100) {
    //     let perm = Percent100::from_permille(permille);
    //     assert_eq!(exp, perm.of(quantity), "Calculating {perm} of {quantity}",);
    // }

    pub(crate) fn test_of<P>(permille: Units, quantity: P, exp: P)
    where
        P: Clone + Debug + Display + Fractionable<Percent100> + PartialEq,
    {
        let perm = Percent100::from_permille(permille);
        assert_eq!(
            exp,
            perm.of(quantity.clone()),
            "Calculating {perm} of {quantity}",
        );
    }

    fn from(permille: Units) -> Percent100 {
        Percent100::from_permille(permille)
    }

    fn test_display(exp: &str, permilles: Units) {
        assert_eq!(exp, format!("{}", Percent100::from_permille(permilles)));
    }
}
