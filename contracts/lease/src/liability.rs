use schemars::JsonSchema;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Copy, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct Liability {
    /// The initial percentage of the amount due versus the locked collateral
    pub init_percent: u8,
    /// The healty percentage of the amount due versus the locked collateral
    pub healthy_percent: u8,
    /// The maximum percentage of the amount due versus the locked collateral
    pub max_percent: u8,
    /// At what time cadence to recalculate the liability
    pub recalc_secs: u32,
}

