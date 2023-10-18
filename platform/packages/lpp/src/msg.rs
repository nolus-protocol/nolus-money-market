use serde::{Deserialize, Serialize};

/// The query message variants each Lpp must implement
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub enum QueryMsg {
    LppBalance(),
}

/// The execute message variants each Lpp must implement
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub enum ExecuteMsg {
    DistributeRewards(),
}
