use bound::BoundPercent;

use crate::{
    error::Error,
    fraction::{Fraction, ToFraction, Unit as FractionUnit},
    fractionable::{CommonDoublePrimitive, Fractionable, IntoMax},
    percent::permilles::Permilles,
    ratio::{Ratio, SimpleFraction},
    rational::Rational,
};

pub(crate) use fractionable::DoubleBoundPercentPrimitive;

pub mod bound;
mod fraction;
mod fractionable;
pub mod permilles;

// TODO Remove once integration tests use BoundPercent::of(Coin)
#[cfg(any(test, feature = "testing"))]
mod units;

pub type Units = u32;
pub type Percent100 = BoundPercent<{ Permilles::MILLE.units() }>;
pub type Percent = BoundPercent<{ Units::MAX }>;

impl Percent100 {
    pub const fn complement(self) -> Self {
        Self::MAX
            .checked_sub(self)
            .expect("Invariant violated: percent is bigger than 100%")
    }

    pub fn from_ratio<U>(parts: U, total: U) -> Self
    where
        Permilles: Fractionable<U>,
        U: FractionUnit + IntoMax<<Permilles as CommonDoublePrimitive<U>>::CommonDouble>,
    {
        debug_assert!(parts <= total);

        Self::try_from(Ratio::new(parts, total).of(Permilles::MILLE))
            .expect("Should be a valid Percent100.")
    }

    fn to_ratio(self) -> Ratio<Permilles> {
        Ratio::new(self.into(), Permilles::MILLE)
    }
}

impl Percent {
    pub fn from_fraction<U>(nominator: U, denominator: U) -> Result<Self, Error>
    where
        Permilles: Fractionable<U>,
        U: FractionUnit + IntoMax<<Permilles as CommonDoublePrimitive<U>>::CommonDouble>,
    {
        let fraction = SimpleFraction::new(nominator, denominator);
        fraction
            .of(Permilles::MILLE)
            .ok_or_else(|| Error::multiplication_overflow(fraction, Permilles::MILLE))
            .and_then(Self::try_from)
    }
}

impl Fraction<Permilles> for Percent100 {
    fn of<A>(&self, whole: A) -> A
    where
        Permilles: IntoMax<A::CommonDouble>,
        A: Fractionable<Permilles>,
    {
        self.to_ratio().of(whole)
    }
}

impl Rational<Permilles> for Percent {
    fn of<A>(&self, whole: A) -> Option<A>
    where
        Permilles: IntoMax<A::CommonDouble>,
        A: Fractionable<Permilles>,
    {
        self.to_fraction().of(whole)
    }
}

impl From<Percent100> for Percent {
    fn from(percent: Percent100) -> Self {
        Self::try_from(Permilles::from(percent))
            .expect("Percent value safely fits in internal representation")
    }
}

impl TryFrom<Percent> for Percent100 {
    type Error = Error;

    fn try_from(percent: Percent) -> Result<Self, Self::Error> {
        Permilles::from(percent).try_into()
    }
}

#[cfg(test)]
pub(super) mod test {
    use std::fmt::{Debug, Display};

    use crate::{
        coin::Amount,
        fraction::Fraction,
        fractionable::{CommonDoublePrimitive, Fractionable, IntoMax},
        percent::{Percent, Percent100, permilles::Permilles},
        ratio::Ratio,
        test::coin,
    };

    use super::Units;

    pub const MILLE_UNITS: Units = Permilles::MILLE.units();

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
        test_of(0, Percent100::MAX, Percent100::ZERO);
        test_of(MILLE_UNITS, Percent100::MAX, Percent100::MAX);
        test_of(100, Percent100::ZERO, Percent100::ZERO);
    }

    #[test]
    fn from_ratio() {
        assert_eq!(
            percent100(750),
            Percent100::from_ratio(coin::coin1(3), coin::coin1(4))
        );
        assert_eq!(
            Percent100::MAX,
            Percent100::from_ratio(coin::coin1(3), coin::coin1(3))
        );
        assert_eq!(
            Percent100::MAX,
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
            Ratio::new(Permilles::ZERO, Permilles::MILLE),
            Percent100::ZERO.to_ratio()
        );
        assert_eq!(
            Ratio::new(Permilles::new(100), Permilles::MILLE),
            percent100(100).to_ratio()
        );
        assert_eq!(
            Ratio::new(Permilles::MILLE, Permilles::MILLE),
            Percent100::MAX.to_ratio()
        );
    }

    #[test]
    fn percent_to_percent100() {
        assert_eq!(percent100(500), percent(500).try_into().unwrap());
        assert_eq!(
            percent100(MILLE_UNITS),
            percent(MILLE_UNITS).try_into().unwrap()
        );
        assert!(Percent100::try_from(percent(1001)).is_err());
    }

    #[test]
    fn from_fraction() {
        let n: Units = 189;
        let d: Units = 1890;
        let res = Percent::from_fraction(Permilles::new(n), Permilles::new(d)).unwrap();
        assert_eq!(percent(n * MILLE_UNITS / d), res);
    }

    pub(crate) fn test_of<P>(permille: Units, quantity: P, exp: P)
    where
        P: Clone + Debug + Display + Fractionable<Permilles> + PartialEq,
        Permilles: IntoMax<<P as CommonDoublePrimitive<Permilles>>::CommonDouble>,
    {
        let perm = percent100(permille);
        assert_eq!(
            exp,
            perm.of(quantity.clone()),
            "Calculating {perm} of {quantity}",
        );
    }

    pub(super) fn percent100(permille: Units) -> Percent100 {
        Percent100::from_permille(permille)
    }

    pub(super) fn percent(permille: Units) -> Percent {
        Percent::from_permille(permille)
    }
}
