use serde::{Deserialize, Serialize};

use currency::native::Nls;
use finance::{
    coin::{Coin, CoinDTO},
    currency::Currency,
    percent::Percent,
    price::Price,
};
use sdk::{
    cosmwasm_std::{Addr, Timestamp, Uint128, Uint64},
    schemars::{self, JsonSchema},
};

use crate::nlpn::NLpn;

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub lpn_ticker: String,
    pub lease_code_id: Uint64,
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    UpdateParameters {
        base_interest_rate: Percent,
        utilization_optimal: Percent,
        addon_optimal_interest_rate: Percent,
    },

    OpenLoan {
        amount: CoinDTO,
    },
    RepayLoan(),

    Deposit(),
    // CW20 interface, withdraw from lender deposit
    Burn {
        amount: Uint128,
    },

    DistributeRewards(),
    ClaimRewards {
        other_recipient: Option<Addr>,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    Config(),
    Quote {
        amount: CoinDTO,
    },
    Loan {
        lease_addr: Addr,
    },
    LoanOutstandingInterest {
        lease_addr: Addr,
        outstanding_time: Timestamp,
    },

    // Deposit
    /// CW20 interface, lender deposit balance
    Balance {
        address: Addr,
    },
    LppBalance(),
    Price(),

    Rewards {
        address: Addr,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct QueryConfigResponse {
    pub lpn_ticker: String,
    pub lease_code_id: Uint64,
    pub base_interest_rate: Percent,
    pub utilization_optimal: Percent,
    pub addon_optimal_interest_rate: Percent,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryQuoteResponse {
    QuoteInterestRate(Percent),
    NoLiquidity,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct LoanResponse<Lpn>
where
    Lpn: Currency,
{
    pub principal_due: Coin<Lpn>,
    pub interest_due: Coin<Lpn>,
    pub annual_interest_rate: Percent,
    pub interest_paid: Timestamp,
}

pub type QueryLoanResponse<Lpn> = Option<LoanResponse<Lpn>>;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct OutstandingInterest<Lpn>(pub Coin<Lpn>)
where
    Lpn: Currency;

pub type QueryLoanOutstandingInterestResponse<Lpn> = Option<OutstandingInterest<Lpn>>;

// Deposit query responses

// CW20 interface
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, JsonSchema, Debug)]
pub struct BalanceResponse {
    pub balance: Uint128,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, JsonSchema, Debug)]
pub struct PriceResponse<LPN>(pub Price<NLpn, LPN>)
where
    LPN: 'static + Currency;

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, JsonSchema, Debug)]
pub struct LppBalanceResponse<LPN>
where
    LPN: Currency,
{
    pub balance: Coin<LPN>,
    pub total_principal_due: Coin<LPN>,
    pub total_interest_due: Coin<LPN>,
    pub balance_nlpn: Coin<NLpn>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, JsonSchema, Debug)]
pub struct RewardsResponse {
    pub rewards: Coin<Nls>,
}
