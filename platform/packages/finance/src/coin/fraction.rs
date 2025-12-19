use std::ops::{Div, Rem};

use gcd::Gcd;

use crate::{
    coin::{Amount, Coin},
    fraction::Unit as FractionUnit,
    zero::Zero,
};

// Used only for average price calculation
impl FractionUnit for Amount {
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

impl<C> FractionUnit for Coin<C> {
    type Times = Amount;

    fn gcd<U>(self, other: U) -> Self::Times
    where
        U: FractionUnit<Times = Self::Times>,
    {
        Gcd::gcd(self.amount, other.to_primitive())
    }

    fn scale_down(self, scale: Self::Times) -> Self {
        debug_assert_ne!(scale, Self::Times::ZERO);

        Coin::new(self.amount.div(scale))
    }

    fn modulo(self, scale: Self::Times) -> Self::Times {
        debug_assert_ne!(scale, Self::Times::ZERO);

        self.amount.rem(scale)
    }

    fn to_primitive(self) -> Self::Times {
        self.amount
    }
}

impl<C> Zero for Coin<C> {
    const ZERO: Self = Self::new(Zero::ZERO);
}
