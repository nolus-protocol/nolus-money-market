use serde::{Deserialize, Serialize};

use finance::percent::Percent;
use sdk::{
    cosmwasm_std::Timestamp,
    schemars::{self, JsonSchema},
};

use super::{LeaseCoin, LpnCoin};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct StateQuery {}

#[derive(Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
#[cfg_attr(any(test, feature = "testing"), derive(Clone, Debug))]
#[serde(rename_all = "snake_case")]
#[allow(clippy::large_enum_variant)] // TODO consider removing it once have added all intermediate states
pub enum StateResponse {
    Opened {
        amount: LeaseCoin,
        interest_rate: Percent,
        interest_rate_margin: Percent,
        principal_due: LpnCoin,
        previous_margin_due: LpnCoin,
        previous_interest_due: LpnCoin,
        current_margin_due: LpnCoin,
        current_interest_due: LpnCoin,
        validity: Timestamp,
    },
    Paid(LeaseCoin),
    Closed(),
}
