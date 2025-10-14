use std::ops::{Div, Rem};

use gcd::Gcd;

use crate::{
    duration::{Duration, Units},
    fraction::Unit as FractionUnit,
    zero::Zero,
};

impl FractionUnit for Duration {
    type Times = Units;

    fn gcd<U>(self, other: U) -> Self::Times
    where
        U: FractionUnit<Times = Self::Times>,
    {
        Gcd::gcd(self.nanos(), other.to_primitive())
    }

    fn scale_down(self, scale: Self::Times) -> Self {
        debug_assert_ne!(scale, Self::Times::ZERO);

        Self::from_nanos(self.nanos().div(scale))
    }

    fn modulo(self, scale: Self::Times) -> Self::Times {
        debug_assert_ne!(scale, Self::Times::ZERO);

        self.nanos().rem(scale)
    }

    fn to_primitive(self) -> Self::Times {
        self.nanos()
    }
}

impl Zero for Duration {
    const ZERO: Self = Self::from_nanos(0);
}
