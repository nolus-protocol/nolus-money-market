use std::ops::{Div, Rem};

use gcd::Gcd;

use crate::{fraction::Unit as FractionUnit, percent::Units, zero::Zero};

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
