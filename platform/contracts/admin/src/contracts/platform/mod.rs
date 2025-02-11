use serde::{Deserialize, Serialize};

use sdk::schemars::{self, JsonSchema};

use super::higher_order_type::FirstOrderType;

#[cfg(feature = "contract")]
mod impl_mod;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(
    rename = "PlatformContracts",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub struct Contracts<T> {
    pub timealarms: T,
    pub treasury: T,
}

impl<T> FirstOrderType<HigherOrderType> for Contracts<T> {
    type Unit = T;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, JsonSchema)]
pub enum HigherOrderType {}

impl super::HigherOrderType for HigherOrderType {
    type Of<Unit> = Contracts<Unit>;
}
