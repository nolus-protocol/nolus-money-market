mod opening;
mod repayment;

pub use opening::{LoanForm, NewLeaseForm, Denom};
pub use repayment::Repay;
use schemars::JsonSchema;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    Repay,
}