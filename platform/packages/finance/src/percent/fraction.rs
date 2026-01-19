use std::ops::{Div, Rem};

use gcd::Gcd;

use crate::{
    fraction::{ToFraction, Unit as FractionUnit},
    percent::{Units, bound::BoundPercent, permilles::Permilles},
    ratio::SimpleFraction,
    zero::Zero,
};

impl FractionUnit for Permilles {
    type Times = Units;

    fn gcd<U>(self, other: U) -> Self::Times
    where
        U: FractionUnit<Times = Self::Times>,
    {
        Gcd::gcd(self.to_primitive(), other.to_primitive())
    }

    fn scale_down(self, scale: Self::Times) -> Self {
        debug_assert_ne!(scale, Self::Times::ZERO);

        Self::new(self.to_primitive().div(scale))
    }

    fn modulo(self, scale: Self::Times) -> Self::Times {
        debug_assert_ne!(scale, Self::Times::ZERO);

        self.to_primitive().rem(scale)
    }

    fn to_primitive(self) -> Self::Times {
        self.units()
    }
}

impl Zero for Permilles {
    const ZERO: Self = Self::ZERO;
}

impl<const UPPER_BOUND: Units> ToFraction<Permilles> for BoundPercent<UPPER_BOUND> {
    fn to_fraction(self) -> SimpleFraction<Permilles> {
        SimpleFraction::new(self.permilles(), Permilles::MILLE)
    }
}

#[cfg(test)]
mod test {
    use crate::{
        fraction::ToFraction,
        percent::{Percent, Percent100, permilles::Permilles, test},
        ratio::SimpleFraction,
    };

    #[test]
    fn to_fraction() {
        assert_eq!(
            SimpleFraction::new(Permilles::ZERO, Permilles::MILLE),
            Percent100::ZERO.to_fraction()
        );
        assert_eq!(
            SimpleFraction::new(Permilles::MILLE, Permilles::MILLE),
            Percent::HUNDRED.to_fraction()
        );
        assert_eq!(
            SimpleFraction::new(Permilles::new(1001), Permilles::MILLE),
            test::percent(1001).to_fraction()
        );
    }
}
