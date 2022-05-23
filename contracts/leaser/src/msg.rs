use cosmwasm_std::{Addr, Coin, Decimal, Uint64};

use finance::percent::Percent;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::config::Config;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub lease_code_id: Uint64,
    pub lpp_ust_addr: Addr,
    pub lease_interest_rate_margin: Percent, // LeaseInterestRateMargin%, for example 3%
    pub recalc_hours: u16,
    pub liability: Liability, // LeaseMaxLiability%, for example 80%
    pub repayment: Repayment, // GracePeriodSec, for example 10 days = 10*24*60*60
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Liability {
    pub max: u8,     // LeaseMaxLiability%, for example 80%
    pub healthy: u8, // LeaseHealthyLiability%, for example, 70%, must be less than LeaseMaxLiability%
    pub initial: u8, // LeaseInitialLiability%, for example, 65%, must be less or equal to LeaseHealthyLiability%
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Repayment {
    pub period_sec: u32,       // PeriodLengthSec, for example 90 days = 90*24*60*60
    pub grace_period_sec: u32, // GracePeriodSec, for example 10 days = 10*24*60*60
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    Config {
        lease_interest_rate_margin: Percent,
        liability: Liability,
        repayment: Repayment,
    },
    OpenLease {
        currency: String,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    Config {},
    Quote { downpayment: Coin },
    Leases { owner: Addr },
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

impl Repayment {
    pub fn new(period_sec: u32, grace_period_sec: u32) -> Self {
        Repayment {
            period_sec,
            grace_period_sec,
        }
    }
}

impl Liability {
    pub fn new(initial: u8, healthy: u8, max: u8) -> Self {
        Liability::validate(initial, healthy, max);

        Liability {
            max,
            healthy,
            initial,
        }
    }

    pub fn validate(initial: u8, healthy: u8, max: u8) {
        assert!(
            healthy < max,
            "LeaseHealthyLiability% must be less than LeaseMaxLiability%"
        );

        assert!(
            initial <= healthy,
            "LeaseInitialLiability% must be less or equal to LeaseHealthyLiability%"
        );
    }
}
