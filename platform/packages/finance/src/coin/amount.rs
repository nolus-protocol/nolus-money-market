use crate::{
    coin::{Amount, DoubleCoinPrimitive},
    fractionable::{IntoMax, ToDoublePrimitive, TryFromMax},
};

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
