use serde::{Deserialize, Serialize};

use finance::percent::bound::BoundToHundredPercent;

/// The query API any contract who implements [PositionLimits] should respond to
#[derive(Serialize, PartialEq, Eq)]
#[cfg_attr(feature = "skel_testing", derive(Debug, Deserialize))]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub enum PositionLimits {
    /// Reply with [MaxSlippage]
    MaxSlippage {},
    // MinAmounts {},
}

/// Response of [PositionLimits::MaxSlippage] query
#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "skel_testing", derive(Debug))]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct MaxSlippage {
    pub liquidation: BoundToHundredPercent,
}
