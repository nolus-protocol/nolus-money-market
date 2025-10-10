use std::ops::{Div, Rem};

use bound::BoundPercent;
use gcd::Gcd;

use crate::{
    error::Error,
    fraction::{Fraction, FractionLegacy, Unit as FractionUnit},
    fractionable::{CommonDoublePrimitive, Fractionable, FractionableLegacy, IntoMax},
    ratio::{Ratio, SimpleFraction},
    rational::{Rational, RationalLegacy},
    zero::Zero,
};

pub mod bound;
mod fractionable;

pub type Units = u32;
pub type Percent100 = BoundPercent<{ Percent::HUNDRED.units() }>;
pub type Percent = BoundPercent<{ Units::MAX }>;

// TODO revisit it's usage after removing FractionLegacy<Units> for Percent100
impl FractionUnit for Units {
    type Times = Self;

    fn gcd<U>(self, other: U) -> Self::Times
    where
        U: FractionUnit<Times = Self::Times>,
    {
        Gcd::gcd(self, other.to_primitive())
    }

    fn scale_down(self, scale: Self::Times) -> Self {
        debug_assert_ne!(scale, Self::Times::ZERO);

        self.div(scale)
    }

    fn modulo(self, scale: Self::Times) -> Self::Times {
        debug_assert_ne!(scale, Self::Times::ZERO);

        self.rem(scale)
    }

    fn to_primitive(self) -> Self::Times {
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
        Self: Fractionable<U>,
        U: FractionUnit + IntoMax<<Self as CommonDoublePrimitive<U>>::CommonDouble>,
    {
        debug_assert!(parts <= total);

        Fraction::of(&Ratio::new(parts, total), Self::HUNDRED)
    }

    fn to_ratio(self) -> Ratio<Self> {
        Ratio::new(self, Self::HUNDRED)
    }
}

impl Percent {
    pub fn from_fraction<U>(nominator: U, denominator: U) -> Option<Self>
    where
        Self: Fractionable<U>,
        U: FractionUnit + IntoMax<<Self as CommonDoublePrimitive<U>>::CommonDouble>,
    {
        Rational::of(&SimpleFraction::new(nominator, denominator), Self::HUNDRED)
    }

    fn to_fraction(self) -> SimpleFraction<Self> {
        SimpleFraction::new(self, Self::HUNDRED)
    }
}

impl Fraction<Self> for Percent100 {
    fn of<A>(&self, whole: A) -> A
    where
        Self: IntoMax<A::CommonDouble>,
        A: Fractionable<Self>,
    {
        // TODO remove the full syntax when removing the FractionLegacy
        Fraction::of(&self.to_ratio(), whole)
    }
}

// TODO remove when implement Fractionable<BoundPercent> for Price
impl FractionLegacy<Units> for Percent100 {
    fn of<A>(&self, whole: A) -> A
    where
        A: FractionableLegacy<Units>,
    {
        FractionLegacy::of(&Ratio::new(self.units(), Self::HUNDRED.units()), whole)
    }
}

impl Rational<Self> for Percent {
    fn of<A>(&self, whole: A) -> Option<A>
    where
        Self: IntoMax<A::CommonDouble>,
        A: Fractionable<Self>,
    {
        // TODO remove the full syntax when removing the RationalLegacy
        Rational::of(&self.to_fraction(), whole)
    }
}

impl RationalLegacy<Units> for Percent {
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

    use crate::{
        coin::Amount,
        fraction::Fraction,
        fractionable::{CommonDoublePrimitive, Fractionable, IntoMax},
        percent::{Percent, Percent100},
        ratio::{Ratio, SimpleFraction},
        rational::Rational,
        test::coin,
    };

    use super::Units;

    #[test]
    fn of() {
        test_of(
            100,
            Percent100::from_percent(40),
            Percent100::from_percent(4),
        );
        test_of(100, Percent100::from_percent(40), percent100(40));
        test_of(10, percent100(800), percent100(8));
        test_of(10, percent100(890), percent100(8));
        test_of(1, percent100(123), Percent100::ZERO);
        test_of(0, Percent100::HUNDRED, Percent100::ZERO);
        test_of(1000, Percent100::HUNDRED, Percent100::HUNDRED);
        test_of(100, Percent100::ZERO, Percent100::ZERO);
    }

    #[test]
    fn from_ratio() {
        assert_eq!(
            percent100(750),
            Percent100::from_ratio(coin::coin1(3), coin::coin1(4))
        );
        assert_eq!(
            Percent100::HUNDRED,
            Percent100::from_ratio(coin::coin1(3), coin::coin1(3))
        );
        assert_eq!(
            Percent100::HUNDRED,
            Percent100::from_ratio(coin::coin1(Amount::MAX), coin::coin1(Amount::MAX))
        );
        assert_eq!(
            percent100(50),
            Percent100::from_ratio(coin::coin1(1), coin::coin1(20))
        );
    }

    #[test]
    fn to_ratio() {
        assert_eq!(
            Ratio::new(Percent100::ZERO, Percent100::HUNDRED),
            Percent100::ZERO.to_ratio()
        );
        assert_eq!(
            Ratio::new(percent100(100), Percent100::HUNDRED),
            percent100(100).to_ratio()
        );
        assert_eq!(
            Ratio::new(Percent100::HUNDRED, Percent100::HUNDRED),
            Percent100::HUNDRED.to_ratio()
        );
    }

    #[test]
    fn to_fraction() {
        assert_eq!(
            SimpleFraction::new(Percent::ZERO, Percent::HUNDRED),
            Percent::ZERO.to_fraction()
        );
        assert_eq!(
            SimpleFraction::new(Percent::HUNDRED, Percent::HUNDRED),
            Percent::HUNDRED.to_fraction()
        );
        assert_eq!(
            SimpleFraction::new(percent(1001), Percent::HUNDRED),
            percent(1001).to_fraction()
        );
    }

    #[test]
    fn percent_to_percent100() {
        assert_eq!(percent100(500), percent(500).try_into().unwrap());
        assert_eq!(percent100(1000), percent(1000).try_into().unwrap());
        assert!(Percent100::try_from(percent(1001)).is_err());
    }

    #[test]
    fn from_fraction() {
        let n: Units = 189;
        let d: Units = 1890;
        let r = SimpleFraction::new(percent(n), percent(d));
        let res = r.of(Percent::HUNDRED).unwrap();
        assert_eq!(percent(n * 1000 / d), res);
    }

    pub(crate) fn test_of<P>(permille: Units, quantity: P, exp: P)
    where
        P: Clone + Debug + Display + Fractionable<Percent100> + PartialEq,
        Percent100: IntoMax<<P as CommonDoublePrimitive<Percent100>>::CommonDouble>,
    {
        let perm = percent100(permille);
        assert_eq!(
            exp,
            perm.of(quantity.clone()),
            "Calculating {perm} of {quantity}",
        );
    }

    fn percent100(permille: Units) -> Percent100 {
        Percent100::from_permille(permille)
    }

    fn percent(permille: Units) -> Percent {
        Percent::from_permille(permille)
    }
}
