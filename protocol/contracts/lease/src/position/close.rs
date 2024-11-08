use finance::percent::Percent;
use serde::{Deserialize, Serialize};

/// Close position policy
///
/// Not designed to be used as an input API component! Invariant checks are not done on deserialization!
///
#[derive(Copy, Clone, Default, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(test, derive(Debug))]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct Policy {
    stop_loss: Option<Percent>,
    take_profit: Option<Percent>,
}

/// A strategy triggered to close the position automatically
///
/// If a recent price movement have the position's LTV trigger a full close as per the configured `Policy`
/// then the close strategy carries details.
///
/// A full close of the position is triggered if:
/// - a Stop Loss is set up and a price decline have the position's LTV become higher than the specified percent, or
/// - a Take Profit is set up and a price rise have the position's LTV become lower than the specified percent.
#[allow(dead_code)]
pub enum Strategy {
    StopLoss(Percent),
    TakeProfit(Percent),
}
