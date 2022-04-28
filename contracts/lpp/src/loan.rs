use cosmwasm_std::{Uint128, Decimal, Timestamp};
use serde::{Serialize, Deserialize};
use schemars::JsonSchema;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Loan {
    pub principal_due: Uint128,
    pub annual_interest_rate: Decimal,
    pub interest_paid: Timestamp,
}
