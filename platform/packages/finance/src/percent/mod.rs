use std::ops::{Div, Rem};

use bound::BoundPercent;
use gcd::Gcd;

use crate::{
    error::Error,
    fraction::{Fraction, Unit as FractionUnit},
    fractionable::FractionableLegacy,
    ratio::{Ratio, SimpleFraction},
    rational::Rational,
    zero::Zero,
};

pub mod bound;

pub type Units = u32;
pub type Percent100 = BoundPercent<{ Percent::HUNDRED.units() }>;
pub type Percent = BoundPercent<{ Units::MAX }>;

impl FractionUnit for Units {
    type Times = Self;

    fn gcd<U>(self, other: U) -> Self::Times
    where
        U: FractionUnit<Times = Self::Times>,
    {
        Gcd::gcd(self, other.primitive())
    }

    fn scale_down(self, scale: Self::Times) -> Self {
        debug_assert_ne!(scale, Self::Times::ZERO);

        self.div(scale)
    }

    fn modulo(self, scale: Self::Times) -> Self::Times {
        debug_assert_ne!(scale, Self::Times::ZERO);

        self.rem(scale)
    }

    fn primitive(self) -> Self::Times {
        self
    }
}

impl Percent100 {
    pub const fn complement(self) -> Self {
        Self::HUNDRED
            .checked_sub(self)
            .expect("Invariant violated: percent is bigger than 100%")
    }

    pub fn from_ratio<U>(parts: U, total: U) -> Self
    where
        Self: FractionableLegacy<U>,
        U: FractionUnit,
    {
        debug_assert!(parts <= total);

        Ratio::new(parts, total).of(Self::HUNDRED)
    }

    fn to_ratio(self) -> Ratio<Units> {
        Ratio::new(self.units(), Self::HUNDRED.units())
    }
}

impl Percent {
    pub fn from_fraction<U>(nominator: U, denominator: U) -> Option<Self>
    where
        Self: FractionableLegacy<U>,
        U: FractionUnit,
    {
        SimpleFraction::new(nominator, denominator).of(Self::HUNDRED)
    }
}

impl Fraction<Units> for Percent100 {
    fn of<A>(&self, whole: A) -> A
    where
        A: FractionableLegacy<Units>,
    {
        self.to_ratio().of(whole)
    }
}

impl Rational<Units> for Percent {
    fn of<A>(&self, whole: A) -> Option<A>
    where
        A: FractionableLegacy<Units>,
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

    use currency::test::SubGroupTestC10;

    use crate::{
        coin::{Amount, Coin},
        fraction::Fraction,
        fractionable::FractionableLegacy,
        percent::{Percent, Percent100},
        ratio::{Ratio, SimpleFraction},
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
    fn from_ratio() {
        assert_eq!(
            Percent100::from_permille(750),
            Percent100::from_ratio(3u32, 4u32)
        );
        assert_eq!(Percent100::HUNDRED, Percent100::from_ratio(3u32, 3u32));
        assert_eq!(
            Percent100::HUNDRED,
            Percent100::from_ratio(
                Coin::<SubGroupTestC10>::new(Amount::MAX),
                Coin::<SubGroupTestC10>::new(Amount::MAX)
            )
        );
        assert_eq!(
            Percent100::from_permille(50),
            Percent100::from_ratio(
                Coin::<SubGroupTestC10>::new(1),
                Coin::<SubGroupTestC10>::new(20)
            )
        );
    }

    #[test]
    fn to_ratio() {
        assert_eq!(Ratio::new(0, 1000), Percent100::ZERO.to_ratio());
        assert_eq!(
            Ratio::new(100, 1000),
            Percent100::from_permille(100).to_ratio()
        );
        assert_eq!(Ratio::new(1000, 1000), Percent100::HUNDRED.to_ratio());
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
        P: Clone + Debug + Display + FractionableLegacy<Units> + PartialEq,
    {
        let perm = Percent100::from_permille(permille);
        assert_eq!(
            exp,
            perm.of(quantity.clone()),
            "Calculating {perm} of {quantity}",
        );
    }
}
