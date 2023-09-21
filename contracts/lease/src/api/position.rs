use serde::{Deserialize, Serialize};

use sdk::schemars::{self, JsonSchema};

use super::LeaseCoin;

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, JsonSchema)]
#[cfg_attr(any(test, feature = "testing"), derive(Debug))]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub enum PositionClose {
    FullClose,
    PartialClose,
}

#[derive(Serialize, Deserialize)]
pub struct FullClose();

#[derive(Serialize, Deserialize)]
pub struct PartialClose {
    pub amount: LeaseCoin,
}
