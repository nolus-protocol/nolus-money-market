use cosmwasm_std::{Addr, Coin as CwCoin, Decimal, Timestamp, Uint128, Uint64};
use finance::{coin::Coin, currency::Currency, percent::Percent};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub denom: String,
    pub lease_code_id: Uint64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    UpdateParameters {
        base_interest_rate: Percent,
        utilization_optimal: Percent,
        addon_optimal_interest_rate: Percent,
    },

    OpenLoan {
        amount: CwCoin,
    },
    RepayLoan,

    Deposit(),
    // CW20 interface, withdraw from lender deposit
    Burn {
        amount: Uint128,
    },

    DistributeRewards,
    ClaimRewards {
        other_recipient: Option<Addr>,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    Config(),
    Quote {
        amount: CwCoin,
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

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct QueryConfigResponse {
    pub lpn_symbol: String,
    pub lease_code_id: Uint64,
    pub base_interest_rate: Percent,
    pub utilization_optimal: Percent,
    pub addon_optimal_interest_rate: Percent,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryQuoteResponse {
    QuoteInterestRate(Percent),
    NoLiquidity,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
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

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct OutstandingInterest<Lpn>(pub Coin<Lpn>)
where
    Lpn: Currency;

pub type QueryLoanOutstandingInterestResponse<Lpn> = Option<OutstandingInterest<Lpn>>;

// Deposit query responses

// CW20 interface
#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct BalanceResponse {
    pub balance: Uint128,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct PriceResponse {
    pub price: Decimal,
    pub denom: String,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct LppBalanceResponse {
    pub balance: CwCoin,
    pub total_principal_due: CwCoin,
    pub total_interest_due: CwCoin,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct RewardsResponse {
    pub rewards: CwCoin,
}
