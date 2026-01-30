use std::{
    fmt::Debug,
    ops::{Div, Rem},
};

use gcd::Gcd;

use crate::{
    coin::Amount,
    fraction::{ToFraction, Unit as FractionUnit},
    percent::{Units, bound::BoundPercent},
    ratio::SimpleFraction,
    zero::Zero,
};

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

impl<const UPPER: Units> FractionUnit for BoundPercent<UPPER>
where
    BoundPercent<UPPER>: Copy + Debug + Ord + Zero,
{
    type Times = Units;

    fn gcd<U>(self, other: U) -> Self::Times
    where
        U: FractionUnit<Times = Self::Times>,
    {
        Gcd::gcd(self.units(), other.to_primitive())
    }

    fn scale_down(self, scale: Self::Times) -> Self {
        debug_assert_ne!(scale, Self::Times::ZERO);
        Self::try_from(self.units().div(scale)).expect("Units should be less than UPPER_BOUND")
    }

    fn modulo(self, scale: Self::Times) -> Self::Times {
        debug_assert_ne!(scale, Self::Times::ZERO);

        self.units().rem(scale)
    }

    fn to_primitive(self) -> Self::Times {
        self.units()
    }
}

impl<const UPPER_BOUND: Units> Zero for BoundPercent<UPPER_BOUND> {
    const ZERO: Self = Self::ZERO;
}

impl<const UPPER_BOUND: Units> ToFraction<Self> for BoundPercent<UPPER_BOUND> {
    fn to_fraction(self) -> SimpleFraction<Self> {
        SimpleFraction::new(self, Self::HUNDRED)
    }
}

impl<const UPPER_BOUND: Units> ToFraction<Amount> for BoundPercent<UPPER_BOUND> {
    fn to_fraction(self) -> SimpleFraction<Amount> {
        SimpleFraction::new(
            self.to_primitive().into(),
            Self::HUNDRED.to_primitive().into(),
        )
    }
}
