use serde::{Deserialize, Serialize};

use finance::coin::Coin;
use sdk::schemars::{self, JsonSchema};

use crate::{NLpn, Usd};

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

#[derive(Serialize, Deserialize, JsonSchema)]
#[cfg_attr(any(test, feature = "testing"), derive(Debug))]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct LppBalanceResponse {
    pub balance: Coin<Usd>,
    pub total_principal_due: Coin<Usd>,
    pub total_interest_due: Coin<Usd>,
    pub balance_nlpn: Coin<NLpn>,
}
