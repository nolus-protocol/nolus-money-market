use serde::{Deserialize, Serialize};

use sdk::schemars::{self, JsonSchema};

pub(crate) mod type_defs;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct CodeIdWithMigrateMsg<M> {
    pub code_id: u64,
    pub migrate_msg: M,
}

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
