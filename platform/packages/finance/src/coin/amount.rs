use bnum::types::U256;

use crate::{
    coin::Amount,
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
impl IntoMax<U256> for Amount {
    fn into_max(self) -> U256 {
        self.to_double()
    }
}

// TODO remove this implemenatation when Price converts to SimpleFraction<Quote, C>
impl ToDoublePrimitive for Amount {
    type Double = U256;

    fn to_double(&self) -> Self::Double {
        U256::from(*self)
    }
}

// TODO remove this implemenatation when Price converts to SimpleFraction<Quote, C>
impl TryFromMax<U256> for Amount {
    fn try_from_max(max: U256) -> Option<Self> {
        max.try_into().ok()
    }
}
