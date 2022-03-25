use cosmwasm_std::{Addr, Coin, Decimal, Timestamp};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub denom: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    Borrow { amount: Coin },
    Repay { amount: Coin },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    Quote {
        loan: Addr,
        amount: Coin,
    },
    Borrow {
        loan: Addr,
    },
    BorrowOutstandingInterest {
        loan: Addr,
        outstanding_by: Timestamp,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct QueryQuoteResponse {
    quote_interest_rate: Decimal,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryBorrowResponse {
    Borrow {
        principal_due: Coin,
        annual_interest_rate: Decimal,
        // NOTE: is it ok to use a Timestamp? or switch to Uint64
        interest_paid_by: Timestamp,
    },
    // NOTE: how about switch to Option<QueryBorrowResponse> for query response?
    BorrowNotFound,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryBorrowOutstandingInterestResponse {
    // NOTE: is Coin ok or better downgrade to Uint128?
    OutstandingInterest(Coin),
    BorrowNotFound,
}
