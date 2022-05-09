use cosmwasm_std::{Addr, Coin, Decimal, Uint256};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::config::Config;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub lease_code_id: u64,
    pub lpp_ust_addr: Addr,
    pub lease_interest_rate_margin: u8, // LeaseInterestRateMargin%, for example 3%
    pub lease_max_liability: u8,        // LeaseMaxLiability%, for example 80%
    pub lease_healthy_liability: u8, // LeaseHealthyLiability%, for example, 70%, must be less than LeaseMaxLiability%
    pub lease_initial_liability: u8, // LeaseInitialLiability%, for example, 65%, must be less or equal to LeaseHealthyLiability%
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
    Quote { downpayment: Coin },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ConfigResponse {
    pub config: Config,
}

// totalUST, borrowUST, annualInterestRate%
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct QuoteResponse {
    pub total: Coin,
    pub borrow: Coin,
    pub annual_interest_rate: Decimal,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum LPPQueryMsg {
    Quote { amount: Coin },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryQuoteResponse {
    QuoteInterestRate(Decimal),
    NoLiquidity,
}
