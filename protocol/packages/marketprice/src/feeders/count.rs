use std::ops::{Div, Rem};

use gcd::Gcd;

use finance::{
    fraction::Unit as FractionUnit,
    fractionable::{CommonDoublePrimitive, Fractionable, IntoMax, ToDoublePrimitive, TryFromMax},
    percent::Percent100,
    ratio::{RatioLegacy, SimpleFraction},
    zero::Zero,
};

use crate::feeders::PriceFeedersError;

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct Count(u32);

impl Count {
    pub const ONE: Self = Self(1);
    pub const MAX: Self = Self(u32::MAX);

    const fn new(count: u32) -> Self {
        Self(count)
    }

    #[cfg(test)]
    pub(crate) const fn new_test(count: u32) -> Self {
        Self::new(count)
    }

    pub fn can_increment(&self) -> Option<()> {
        (self != &Self::MAX).then_some(())
    }

    pub fn try_into_reciproral(self) -> Option<impl RatioLegacy<Self>> {
        (self.0 != 0).then_some(SimpleFraction::new(Self::ONE, self))
    }
}

impl TryFrom<usize> for Count {
    type Error = PriceFeedersError;

    fn try_from(value: usize) -> Result<Self, Self::Error> {
        value
            .try_into()
            .map_err(Self::Error::MaxFeederCount)
            .map(Self::new)
    }
}

impl From<Count> for u128 {
    fn from(val: Count) -> Self {
        val.0.into()
    }
}

impl CommonDoublePrimitive<Percent100> for Count {
    type CommonDouble = <Count as ToDoublePrimitive>::Double;
}

impl Fractionable<Percent100> for Count {}

impl IntoMax<<Count as CommonDoublePrimitive<Percent100>>::CommonDouble> for Count {
    fn into_max(self) -> <Count as ToDoublePrimitive>::Double {
        self.to_double()
    }
}

impl ToDoublePrimitive for Count {
    type Double = u64;

    fn to_double(&self) -> Self::Double {
        self.0.into()
    }
}

impl TryFromMax<<Count as ToDoublePrimitive>::Double> for Count {
    fn try_from_max(max: <Count as ToDoublePrimitive>::Double) -> Option<Self> {
        max.try_into().map(Self::new).ok()
    }
}

impl FractionUnit for Count {
    type Times = u32;

    fn gcd<U>(self, other: U) -> Self::Times
    where
        U: FractionUnit<Times = Self::Times>,
    {
        Gcd::gcd(self.0, other.to_primitive())
    }

    fn scale_down(self, scale: Self::Times) -> Self {
        debug_assert_ne!(scale, Self::Times::ZERO);

        Self::new(self.0.div(scale))
    }

    fn modulo(self, scale: Self::Times) -> Self::Times {
        debug_assert_ne!(scale, Self::Times::ZERO);

        self.0.rem(scale)
    }

    fn to_primitive(self) -> Self::Times {
        self.0
    }
}

impl Zero for Count {
    const ZERO: Self = Self::new(0);
}
