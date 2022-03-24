use cosmwasm_std::{Addr, Decimal256, Uint256};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use crate::state::Config;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub loan_code_id: u64,
    pub lpp_ust_addr: Addr,
    pub loan_interest_rate_margin: Decimal256,
    pub loan_max_liability: Decimal256,
    pub loan_healthy_liability: Decimal256,
    pub repayment_period_nano_sec: Uint256,
    pub grace_period_nano_sec: Uint256,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    Borrow {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    Config {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ConfigResponse {
    pub config: Config,
}
