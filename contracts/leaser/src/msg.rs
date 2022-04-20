use crate::state::Config;
use cosmwasm_std::{Addr, Uint256};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub lease_code_id: u64,
    pub lpp_ust_addr: Addr,
    pub lease_interest_rate_margin: u64, // LeaseInterestRateMargin%, for example 3%
    pub lease_max_liability: u64,        // LeaseMaxLiability%, for example 80%
    pub lease_healthy_liability: u64, // LeaseHealthyLiability%, for example, 70%, must be less than LeaseMaxLiability%
    pub lease_initial_liability: u64, // LeaseInitialLiability%, for example, 65%, must be less or equal to LeaseHealthyLiability%
    pub repayment_period_nano_sec: Uint256, // PeriodLengthNanoSec, for example 90 days = 90*24*60*60*1000*1000*1000
    pub grace_period_nano_sec: Uint256, // GracePeriodNanoSec, for example 10 days = 10*24*60*60*1000*1000*1000
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
