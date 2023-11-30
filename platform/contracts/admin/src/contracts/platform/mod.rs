use serde::{Deserialize, Serialize};

use sdk::schemars::{self, JsonSchema};

#[cfg(feature = "contract")]
mod impl_mod;

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct Platform<T> {
    pub dispatcher: T,
    pub timealarms: T,
    pub treasury: T,
}
