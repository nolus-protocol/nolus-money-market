use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::SimpleRule;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteHookMsg {
    Notify(SimpleRule),
}
