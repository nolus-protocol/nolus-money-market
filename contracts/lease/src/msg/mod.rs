mod opening;
mod query;

use marketprice::feed::DenomToPrice;
pub use opening::{Denom, LoanForm, NewLeaseForm};
pub use query::{State, StatusQuery, StatusResponse};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    Repay(), // it is not an enum variant to represent it as a JSON object instead of JSON string
    Close(), // that is a limitation of cosmjs library
    Alarm(DenomToPrice),
}
