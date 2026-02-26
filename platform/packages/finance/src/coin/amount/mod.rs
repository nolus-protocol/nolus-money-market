use crate::{
    coin::{Amount, DoubleCoinPrimitive},
    fractionable::{IntoDoublePrimitive, IntoMax, TryFromMax},
};

pub(super) mod serde;
#[cfg(any(test, feature = "testing"))]
mod testing;

impl IntoDoublePrimitive for Amount {
    type Double = DoubleCoinPrimitive;

    fn into_double(self) -> Self::Double {
        DoubleCoinPrimitive::from(self)
    }
}

impl IntoMax<DoubleCoinPrimitive> for Amount {
    fn into_max(self) -> DoubleCoinPrimitive {
        self.into_double()
    }
}

impl TryFromMax<DoubleCoinPrimitive> for Amount {
    fn try_from_max(max: DoubleCoinPrimitive) -> Option<Self> {
        max.try_into().ok()
    }
}
