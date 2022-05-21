mod closing;
mod opening;
mod repayment;

pub use closing::Close;
pub use opening::{Denom, LoanForm, NewLeaseForm};
pub use repayment::Repay;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    Repay,
    Close,
}
