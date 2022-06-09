mod opening;
mod query;

pub use opening::{Denom, LoanForm, NewLeaseForm};
pub use query::{StatusQuery, StatusResponse, State};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    Repay(), // it is not an enum variant to represent is as a JSON object instead of JSON string
    Close(), // that is a limitation of cosmjs library
}
