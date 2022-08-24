use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use marketprice::storage::Price;

pub use self::{
    opening::{LoanForm, NewLeaseForm},
    query::{StateQuery, StateResponse}
};

mod opening;
mod query;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    Repay(), // it is not an enum variant to represent it as a JSON object instead of JSON string
    Close(), // that is a limitation of cosmjs library
    PriceAlarm(Price),
}
