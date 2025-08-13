use std::{fmt::Formatter, ops::Add};

use bound::BoundPercent;

use crate::{
    error::{Error, Result as FinanceResult}, fraction::Fraction, fractionable::Fractionable, ratio::{RatioLegacy, SimpleFraction}, traits::FractionUnit, zero::Zero
};

pub mod bound;

pub const MAX_BOUND: Units = Units::MAX;

pub type Units = u32;
pub type Percent100 = BoundPercent<1000>;
pub type Percent = BoundPercent<MAX_BOUND>;

impl FractionUnit for Units {}

impl Fraction<Units> for Percent100 {
    fn of<A>(&self, whole: A) -> A
    where
        A: Fractionable<Units>,
    {
        let fraction = SimpleFraction::<Units>::from(*self);
        fraction
            .of(whole)
            .expect("TODO it won't be needed when ratio.of()")
    }
}

impl Rational<Units> for Percent {
    fn of<A>(&self, whole: A) -> Option<A>
    where
        A: Fractionable<Units>,
    {
        whole.safe_mul(self)
    }
}

impl RatioLegacy<Units> for Percent {
    fn parts(&self) -> Units {
        self.units()
    }

    fn total(&self) -> Units {
        Percent::HUNDRED.units()
    }
}

impl RatioLegacy<Units> for SimpleFraction<Percent> {
    fn parts(&self) -> Units {
        RatioLegacy::<Percent>::parts(self).units()
    }

    fn total(&self) -> Units {
        RatioLegacy::<Percent>::total(self).units()
    }
}

// TODO replace this convertion with From<Percent100> for Ratio after Ratio becomes a struct
impl From<Percent100> for SimpleFraction<Units> {
    fn from(percent: Percent100) -> Self {
        Self::new(percent.units(), Percent100::HUNDRED.units())
    }
}

impl From<Percent> for SimpleFraction<Units> {
    fn from(percent: Percent) -> Self {
        Self::new(percent.units(), Percent::HUNDRED.units())
    }
}

impl From<Percent100> for Percent {
    fn from(percent: Percent100) -> Self {
        Self::try_from(percent.units())
            .expect("Percent value safely fits in internal representation")
    }
}

impl TryFrom<Percent> for Percent100 {
    type Error = Error;

    fn try_from(percent: Percent) -> Result<Self, Self::Error> {
        let permilles = percent.units();
        permilles.try_into()
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
        assert!(Percent100::HUNDRED.checked_add(from(1)).is_none());
        assert!(
            Percent::from_permille(Units::MAX)
                .checked_add(Percent::from_permille(1))
                .is_none()
        );
    }

    #[test]
    fn sub() {
        assert_eq!(from(67), from(79) - (from(12)));
        assert_eq!(from(0), from(34) - (from(34)));
        assert_eq!(from(39), from(39) - (from(0)));
        assert_eq!(from(990), Percent100::HUNDRED - (from(10)));
        assert_eq!(
            from(0),
            Percent100::HUNDRED - Percent100::from_permille(1000)
        );
    }

    #[test]
    fn sub_overflow() {
        assert!(from(34).checked_sub(from(35)).is_none())
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
        test_of(0, Percent100::HUNDRED, Percent100::ZERO);
        test_of(1000, Percent100::HUNDRED, Percent100::HUNDRED);
        test_of(100, Percent100::ZERO, Percent100::ZERO);
    }

    #[test]
    fn percent_to_percent100() {
        assert_eq!(
            Percent100::from_permille(500),
            Percent::from_permille(500).try_into().unwrap()
        );
        assert_eq!(
            Percent100::from_permille(1000),
            Percent::from_permille(1000).try_into().unwrap()
        );
        assert!(Percent100::try_from(Percent::from_permille(1001)).is_err());
    }

    #[test]
    fn rational_to_percents() {
        let n: Units = 189;
        let d: Units = 1890;
        let r = SimpleFraction::new(n, d);
        let res: Percent = r.of(Percent::HUNDRED).unwrap();
        assert_eq!(Percent::from_permille(n * 1000 / d), res);
    }

    pub(crate) fn test_of<P>(permille: Units, quantity: P, exp: P)
    where
        P: Clone + Debug + Display + Fractionable<Units> + PartialEq,
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
