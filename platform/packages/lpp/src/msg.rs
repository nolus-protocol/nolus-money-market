use serde::{Deserialize, Serialize};

use finance::coin::Coin;
use sdk::schemars::{self, JsonSchema};

use crate::{CoinUsd, NLpn};

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
#[cfg_attr(any(test, feature = "testing"), derive(Debug, Clone))]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct LppBalanceResponse {
    pub balance: CoinUsd,
    pub total_principal_due: CoinUsd,
    pub total_interest_due: CoinUsd,
    pub balance_nlpn: Coin<NLpn>,
}
