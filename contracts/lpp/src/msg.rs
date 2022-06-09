use cosmwasm_std::{Addr, Coin, Decimal, Timestamp, Uint64, Uint128};
use finance::percent::Percent;
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
    OpenLoan { amount: Coin },
    RepayLoan,

    Deposit,
    // CW20 interface, withdraw from lender deposit
    Burn { amount: Uint128},

    DistributeRewards,
    ClaimRewards { other_recipient: Option<Addr> },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    Quote {
        amount: Coin,
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
    Balance { address: Addr },
    LppBalance,
    Price,

    Rewards { address: Addr },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryQuoteResponse {
    QuoteInterestRate(Percent),
    NoLiquidity,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct LoanResponse {
    pub principal_due: Coin,
    pub interest_due: Coin,
    pub annual_interest_rate: Percent,
    pub interest_paid: Timestamp,
}

pub type QueryLoanResponse = Option<LoanResponse>;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct OutstandingInterest(pub Coin);

pub type QueryLoanOutstandingInterestResponse = Option<OutstandingInterest>;

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
    pub balance: Coin,
    pub total_principal_due: Coin,
    pub total_interest_due: Coin,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct RewardsResponse {
    pub rewards: Coin
}
