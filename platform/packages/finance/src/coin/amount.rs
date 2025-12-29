use crate::{
    coin::{Amount, DoubleCoinPrimitive},
    fractionable::{IntoMax, ToDoublePrimitive, TryFromMax},
};

#[cfg(any(test, feature = "testing"))]
use crate::{
    fractionable::{CommonDoublePrimitive, Fractionable},
    percent::{Units as PercentUnits, bound::BoundPercent},
};

// TODO Remove once integration tests use BoundPercent::of(Coin)
#[cfg(any(test, feature = "testing"))]
impl<const UPPER_BOUND: PercentUnits> CommonDoublePrimitive<BoundPercent<UPPER_BOUND>> for Amount {
    type CommonDouble = <Self as ToDoublePrimitive>::Double;
}

// TODO Remove once integration tests use BoundPercent::of(Coin)
#[cfg(any(test, feature = "testing"))]
impl<const UPPER_BOUND: PercentUnits> Fractionable<BoundPercent<UPPER_BOUND>> for Amount {}

// TODO remove this implemenatation when Price converts to SimpleFraction<Quote, C>
impl IntoMax<DoubleCoinPrimitive> for Amount {
    fn into_max(self) -> DoubleCoinPrimitive {
        self.to_double()
    }
}

// TODO remove this implemenatation when Price converts to SimpleFraction<Quote, C>
impl ToDoublePrimitive for Amount {
    type Double = DoubleCoinPrimitive;

    fn to_double(&self) -> Self::Double {
        DoubleCoinPrimitive::from(*self)
    }
}

// TODO remove this implemenatation when Price converts to SimpleFraction<Quote, C>
impl TryFromMax<DoubleCoinPrimitive> for Amount {
    fn try_from_max(max: DoubleCoinPrimitive) -> Option<Self> {
        max.try_into().ok()
    }
}
