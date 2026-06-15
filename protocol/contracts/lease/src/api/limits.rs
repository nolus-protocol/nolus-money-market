use serde::{Deserialize, Serialize};

use dex::MaxSlippage;

/// The query API any contract who implements [PositionLimits] should respond to
#[derive(Serialize, PartialEq, Eq)]
#[cfg_attr(feature = "skel_testing", derive(Debug, Deserialize))]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub enum PositionLimits {
    /// Reply with [MaxSlippages]
    MaxSlippages {},
    // MinAmounts {},
}

/// Response of [PositionLimits::MaxSlippages] query
#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "skel_testing", derive(Debug))]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct MaxSlippages {
    /// Bounds the opening swap — buying the lease asset at open.
    pub opening: MaxSlippage,
    /// Bounds the liquidation swap — a forced sell under time pressure.
    pub liquidation: MaxSlippage,
}
