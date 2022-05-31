mod closing;
mod opening;
mod repayment;
mod query;

pub use closing::Close;
pub use opening::{Denom, LoanForm, NewLeaseForm};
pub use repayment::Repay;
pub use query::{StatusQuery, StatusResponse, State};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    Repay,
    Close,
}
