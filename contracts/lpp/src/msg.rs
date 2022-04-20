use cosmwasm_std::{Addr, Coin, Decimal, Timestamp, Uint64};
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
    Loan { amount: Coin },
    Repay { amount: Coin },
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
        outstanding_by: Timestamp,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryQuoteResponse {
    QuoteInterestRate(Decimal),
    NoLiquidity,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryLoanResponse {
    Loan {
        principal_due: Coin,
        annual_interest_rate: Decimal,
        // NOTE: is it ok to use a Timestamp? or switch to Uint64
        interest_paid_by: Timestamp,
    },
    // NOTE: how about switch to Option<QueryLoanResponse> for query response?
    LoanNotFound,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryLoanOutstandingInterestResponse {
    // NOTE: is Coin ok or better downgrade to Uint128?
    OutstandingInterest(Coin),
    LoanNotFound,
}
