use finance::percent::Percent;
use serde::{Deserialize, Serialize};

/// Close position policy
///
/// Designed to be used as a non-public API component! Invariant checks are not done on deserialization!
///
#[derive(Copy, Clone, Default, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(test, derive(Debug))]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct Policy {
    stop_loss: Option<Percent>,
    take_profit: Option<Percent>,
}
