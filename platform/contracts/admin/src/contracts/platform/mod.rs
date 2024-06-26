use serde::{Deserialize, Serialize};

use sdk::schemars::{self, JsonSchema};

#[cfg(feature = "contract")]
mod impl_mod;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct HigherOrderType;

impl super::HigherOrderType for HigherOrderType {
    type Of<T> = PlatformContracts<T>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct PlatformContracts<T> {
    pub timealarms: T,
    pub treasury: T,
}
