use serde::{Deserialize, Serialize};

use finance::{
    coin::CoinDTO, currency::SymbolOwned, duration::Duration, liability::Liability,
    percent::Percent,
};
use sdk::{
    cosmwasm_std::{Addr, Uint64},
    schemars::{self, JsonSchema},
};

use crate::{state::config::Config, ContractError};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct InstantiateMsg {
    pub lease_code_id: Uint64,
    pub lpp_ust_addr: Addr,
    /// LeaseInterestRateMargin%, for example 3%
    pub lease_interest_rate_margin: Percent,
    /// LeaseMaxLiability%, for example 80%
    pub liability: Liability,
    /// GracePeriodSec, for example 10 days = 10*24*60*60
    pub repayment: Repayment,
    pub time_alarms: Addr,
    pub market_price_oracle: Addr,
    pub profit: Addr,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct Repayment {
    /// PeriodLengthSec, for example 90 days = 90*24*60*60
    pub period: Duration,
    /// GracePeriodSec, for example 10 days = 10*24*60*60
    pub grace_period: Duration,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    Config {
        lease_interest_rate_margin: Percent,
        liability: Liability,
        repayment: Repayment,
    },
    OpenLease {
        currency: SymbolOwned,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    Config {},
    Quote {
        downpayment: CoinDTO,
        lease_asset: SymbolOwned,
    },
    Leases {
        owner: Addr,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct ConfigResponse {
    pub config: Config,
}

// totalUST, borrowUST, annualInterestRate%
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct QuoteResponse {
    pub total: CoinDTO,
    pub borrow: CoinDTO,
    pub annual_interest_rate: Percent,
    pub annual_interest_rate_margin: Percent,
}

impl Repayment {
    pub fn new(period_time: Duration, grace_period_time: Duration) -> Self {
        Repayment {
            period: period_time,
            grace_period: grace_period_time,
        }
    }

    pub fn validate_period(&self) -> Result<(), ContractError> {
        if self.period > self.grace_period {
            Ok(())
        } else {
            Err(ContractError::validation_err::<Repayment>(String::from(
                "Period length should be greater than grace period",
            )))
        }
    }
}
