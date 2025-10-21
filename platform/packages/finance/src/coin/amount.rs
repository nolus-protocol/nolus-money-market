use bnum::types::U256;

use crate::{
    coin::Amount,
    fractionable::{IntoMax, ToDoublePrimitive, TryFromMax},
};

// TODO remove this implemenatation when Price converts to SimpleFraction<Quote, C>
impl ToDoublePrimitive for Amount {
    type Double = U256;

    fn to_double(&self) -> Self::Double {
        U256::from(*self)
    }
}

// TODO remove this implemenatation when Price converts to SimpleFraction<Quote, C>
impl IntoMax<U256> for Amount {
    fn into_max(self) -> U256 {
        self.to_double()
    }
}

// TODO remove this implemenatation when Price converts to SimpleFraction<Quote, C>
impl TryFromMax<U256> for Amount {
    fn try_from_max(max: U256) -> Option<Self> {
        max.try_into().ok()
    }
}
