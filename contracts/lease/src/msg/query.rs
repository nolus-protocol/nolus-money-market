use cosmwasm_std::Coin;
use finance::percent::Percent;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct StatusQuery {}

pub type StatusResponse = Option<State>;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct State {
    pub amount: Coin,
    pub annual_interest: Percent,
    pub principal_due: Coin,
    pub interest_due: Coin,
}