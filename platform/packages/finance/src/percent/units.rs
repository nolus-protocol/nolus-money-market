use crate::fractionable::{IntoDoublePrimitive, IntoMax, TryFromMax};
use crate::percent::{DoubleBoundPercentPrimitive, Units};
use crate::{
    fractionable::{CommonDoublePrimitive, Fractionable},
    percent::bound::BoundPercent,
};

// TODO Remove once integration tests use BoundPercent::of(Coin)
impl<const UPPER_BOUND: Units> CommonDoublePrimitive<BoundPercent<UPPER_BOUND>> for Units {
    type CommonDouble = DoubleBoundPercentPrimitive;
}

impl<const UPPER_BOUND: Units> Fractionable<BoundPercent<UPPER_BOUND>> for Units {}

impl IntoMax<DoubleBoundPercentPrimitive> for Units {
    fn into_max(self) -> DoubleBoundPercentPrimitive {
        self.into_double()
    }
}

impl IntoDoublePrimitive for Units {
    type Double = DoubleBoundPercentPrimitive;

    fn into_double(self) -> Self::Double {
        DoubleBoundPercentPrimitive::from(self)
    }
}

impl TryFromMax<DoubleBoundPercentPrimitive> for Units {
    fn try_from_max(max: DoubleBoundPercentPrimitive) -> Option<Self> {
        max.try_into().ok()
    }
}
