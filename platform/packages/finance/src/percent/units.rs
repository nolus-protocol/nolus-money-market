use crate::fractionable::{IntoMax, ToDoublePrimitive, TryFromMax};
use crate::percent::Units;
use crate::{
    fractionable::{CommonDoublePrimitive, Fractionable},
    percent::bound::BoundPercent,
};

// TODO Remove once integration tests use BoundPercent::of(Coin)
impl<const UPPER_BOUND: Units> CommonDoublePrimitive<BoundPercent<UPPER_BOUND>> for Units {
    type CommonDouble = <Self as ToDoublePrimitive>::Double;
}

impl<const UPPER_BOUND: Units> Fractionable<BoundPercent<UPPER_BOUND>> for Units {}

impl IntoMax<u64> for Units {
    fn into_max(self) -> u64 {
        self.to_double()
    }
}

impl ToDoublePrimitive for Units {
    type Double = u64;

    fn to_double(self) -> Self::Double {
        u64::from(self)
    }
}

impl TryFromMax<u64> for Units {
    fn try_from_max(max: u64) -> Option<Self> {
        max.try_into().ok()
    }
}
