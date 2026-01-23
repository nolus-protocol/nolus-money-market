use crate::{
    coin::{Amount, DoubleCoinPrimitive},
    fractionable::{IntoMax, ToDoublePrimitive, TryFromMax},
};

pub(super) mod serde;
#[cfg(any(test, feature = "testing"))]
mod testing;

impl IntoMax<DoubleCoinPrimitive> for Amount {
    fn into_max(self) -> DoubleCoinPrimitive {
        self.to_double()
    }
}

impl ToDoublePrimitive for Amount {
    type Double = DoubleCoinPrimitive;

    fn to_double(self) -> Self::Double {
        DoubleCoinPrimitive::from(self)
    }
}

impl TryFromMax<DoubleCoinPrimitive> for Amount {
    fn try_from_max(max: DoubleCoinPrimitive) -> Option<Self> {
        max.try_into().ok()
    }
}
