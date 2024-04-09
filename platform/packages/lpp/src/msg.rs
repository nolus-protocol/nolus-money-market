use serde::{Deserialize, Serialize};

use sdk::schemars::{self, JsonSchema};

use crate::CoinStable;

/// The query message variants each Lpp must implement
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub enum QueryMsg {
    /// Return the total value of a pool in a stable currency
    // TODO add oracle: OracleRef
    StableBalance(),
}

/// The execute message variants each Lpp must implement
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub enum ExecuteMsg {
    DistributeRewards(),
}

#[derive(Serialize, Deserialize, JsonSchema)]
#[cfg_attr(any(test, feature = "testing"), derive(Debug, Clone))]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct StableBalanceResponse {
    pub balance: CoinStable,
    pub total_principal_due: CoinStable,
    pub total_interest_due: CoinStable,
}
