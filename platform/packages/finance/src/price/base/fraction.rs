use bnum::types::U256;

use crate::{fraction::Unit as FractionUnit, zero::Zero};

impl FractionUnit for U256 {
    type Times = Self;

    fn gcd<U>(self, other: U) -> Self::Times
    where
        U: FractionUnit<Times = Self::Times>,
    {
        num_integer::gcd(self, other.to_primitive())
    }

    fn scale_down(self, scale: Self::Times) -> Self {
        self.div(scale)
    }

    fn modulo(self, scale: Self::Times) -> Self::Times {
        self.rem(scale)
    }

    fn to_primitive(self) -> Self::Times {
        self
    }
}

impl Zero for U256 {
    const ZERO: Self = Self::ZERO;
}
