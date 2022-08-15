mod opening;
mod query;

pub use opening::{LoanForm, NewLeaseForm};
pub use query::{StateQuery, StateResponse};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    Repay(), // it is not an enum variant to represent it as a JSON object instead of JSON string
    Close(), // that is a limitation of cosmjs library
}
