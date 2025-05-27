use serde::{Deserialize, Serialize};

use finance::{duration::Duration, percent::Percent};
use lease::api::{limits::MaxSlippages, open::PositionSpecDTO};

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "testing", derive(Debug))]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct NewConfig {
    pub lease_interest_rate_margin: Percent,
    pub lease_position_spec: PositionSpecDTO,
    pub lease_due_period: Duration,
    pub lease_max_slippages: MaxSlippages,
}
