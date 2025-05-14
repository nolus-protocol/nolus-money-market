use serde::{Deserialize, Serialize};

use finance::percent::Percent;

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
#[derive(Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "skel_testing", derive(Clone, Debug))]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct MaxSlippage {
    //   make sure this value is limited to 100%
    // We do not pollute the code with extra validation on deserialization
    // since the new type Percent100 is comming.
    pub liquidation: Percent,
}
