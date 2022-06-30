use cosmwasm_std::Coin;
use finance::percent::Percent;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct StateQuery {}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum StateResponse {
    Opened {
        amount: Coin,
        interest_rate: Percent,
        principal_due: Coin,
        interest_due: Coin,
    },
    Paid(Coin),
    Closed(),
}
