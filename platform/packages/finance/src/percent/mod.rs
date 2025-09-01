use bound::BoundPercent;

use crate::{
    error::Error,
    fraction::{Fraction, Unit as FractionUnit},
    fractionable::Fractionable,
    rational::Rational,
};

pub mod bound;

pub type Units = u32;
pub type Percent100 = BoundPercent<{ Percent::HUNDRED.units() }>;
pub type Percent = BoundPercent<{ Units::MAX }>;

impl FractionUnit for Units {}

impl Percent100 {
    pub const fn complement(self) -> Self {
        Self::HUNDRED
            .checked_sub(self)
            .expect("Invariant violated: percent is bigger than 100%")
    }
}

impl Fraction<Units> for Percent100 {
    fn of<A>(&self, whole: A) -> A
    where
        A: Fractionable<Units>,
    {
        // TODO replace this convertion with From<Percent100> for Ratio after Ratio becomes a struct
        Percent::from(*self)
            .of(whole)
            .expect("TODO it won't be needed when ratio.of()")
    }
}

impl Rational<Units> for Percent {
    fn of<A>(&self, whole: A) -> Option<A>
    where
        A: Fractionable<Units>,
    {
        Some(whole.safe_mul(self))
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
        percent.units().try_into()
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
        let res: Percent = Rational::<Units>::of(&r, Percent::HUNDRED).unwrap();
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
