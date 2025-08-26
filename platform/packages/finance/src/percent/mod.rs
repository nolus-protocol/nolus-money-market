use std::{fmt::Formatter, ops::Add};

use bound::BoundPercent;

use crate::{
    error::{Error, Result as FinanceResult}, fraction::Fraction, fractionable::Fractionable, ratio::{RatioLegacy, SimpleFraction}, traits::FractionUnit, zero::Zero
};

pub mod bound;

pub type Units = u32;
pub type Percent100 = BoundPercent<{ Percent::HUNDRED.units() }>;
pub type Percent = BoundPercent<{ Units::MAX }>;

impl FractionUnit for Units {}

impl Percent100 {
    pub const fn complement(self) -> Self {
        Percent100::HUNDRED
            .checked_sub(self)
            .expect("Invariant violated: percent is bigger than 100%")
    }
}

impl Fraction<Units> for Percent100 {
    fn of<A>(&self, whole: A) -> A
    where
        A: Fractionable<Units>,
    {
        SimpleFraction::from(*self)
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

    use crate::{
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
