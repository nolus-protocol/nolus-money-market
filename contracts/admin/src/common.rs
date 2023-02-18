use serde::{Deserialize, Serialize};

use sdk::schemars::{self, JsonSchema};

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct GeneralContractsGroup<T> {
    pub profit: T,
    pub timealarms: T,
    pub treasury: T,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct SpecializedContractsGroup<T> {
    pub dispatcher: T,
    pub leaser: T,
    pub lpp: T,
    pub oracle: T,
}
