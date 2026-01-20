use std::ops::{Div, Rem};

use gcd::Gcd;

use crate::{
    fraction::Unit as FractionUnit,
    fractionable::{CommonDoublePrimitive, Fractionable, IntoMax, ToDoublePrimitive, TryFromMax},
    percent::{BoundPercent, DoubleBoundPercentPrimitive, Units},
    zero::Zero,
};

// TODO Remove once integration tests use BoundPercent::of(Coin)
impl<const UPPER_BOUND: Units> CommonDoublePrimitive<BoundPercent<UPPER_BOUND>> for Units {
    type CommonDouble = DoubleBoundPercentPrimitive;
}

impl<const UPPER_BOUND: Units> Fractionable<BoundPercent<UPPER_BOUND>> for Units {}

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

impl IntoMax<DoubleBoundPercentPrimitive> for Units {
    fn into_max(self) -> DoubleBoundPercentPrimitive {
        self.to_double()
    }
}

impl ToDoublePrimitive for Units {
    type Double = DoubleBoundPercentPrimitive;

    fn to_double(self) -> Self::Double {
        DoubleBoundPercentPrimitive::from(self)
    }
}

impl TryFromMax<DoubleBoundPercentPrimitive> for Units {
    fn try_from_max(max: DoubleBoundPercentPrimitive) -> Option<Self> {
        max.try_into().ok()
    }
}
