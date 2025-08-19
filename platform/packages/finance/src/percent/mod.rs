use bound::BoundPercent;

use crate::{
    error::Error,
    fraction::{Fraction, Unit as FractionUnit},
    fractionable::Fractionable,
    ratio::SimpleFraction,
    rational::Rational,
};

pub mod bound;

pub type Units = u32;
pub type Percent100 = BoundPercent<1000>;
pub type Percent = BoundPercent<{ Units::MAX }>;

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
        let fraction: SimpleFraction<Units> = SimpleFraction::<Units>::from(*self);
        fraction.of(whole)
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

    use currency::test::SuperGroupTestC1;

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
    fn of_percent() {
        assert_eq!(
            Percent100::from_permille(410 * 222 / 1000),
            Percent100::from_percent(41).of(Percent100::from_permille(222))
        );
        assert_eq!(
            Percent100::from_permille(999),
            Percent100::from_percent(100).of(Percent100::from_permille(999))
        );
        assert_eq!(
            Percent::from_permille(410 * 222222 / 1000),
            Percent::from_percent(41)
                .of(Percent::from_permille(222222))
                .unwrap()
        );
        assert_eq!(
            Percent::from_permille(Units::MAX),
            Percent::from_percent(100)
                .of(Percent::from_permille(Units::MAX))
                .unwrap()
        );

        let p_units: Units = 410;
        let p64: u64 = p_units.into();
        let p64_res = p64 * u64::from(Units::MAX) / 1000;
        let p_units_res: Units = p64_res.try_into().expect("u64 -> Units overflow");

        assert_eq!(
            Percent::from_permille(p_units_res),
            Percent::from_percent(41)
                .of(Percent::from_permille(Units::MAX))
                .unwrap()
        );
    }

    #[test]
    fn of_one() {
        // TODO replace SimpleFraction with Ratio whe it becomes a struct
        assert_eq!(
            Percent100::from_permille(899),
            SimpleFraction::new(
                Coin::<SuperGroupTestC1>::new(u128::MAX),
                Coin::<SuperGroupTestC1>::new(u128::MAX),
            )
            .of(Percent100::from_permille(899))
            .unwrap()
        );
    }

    #[test]
    #[should_panic]
    fn of_overflow() {
        // TODO remove the `#[should_panic]` and assert that is None when
        // SimpleFraction::of() calls its checked_mul method instead of safe_mul
        Percent::from_permille(1001).of(Percent::from_permille(Units::MAX));
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
}
