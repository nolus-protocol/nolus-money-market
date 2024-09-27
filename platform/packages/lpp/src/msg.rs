use serde::{Deserialize, Serialize};

use sdk::cosmwasm_std::Addr;

/// The query message variants each Lpp must implement
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub enum QueryMsg {
    /// Return the total value of a pool in a stable currency as [CoinDTO<PlatformGroup>]
    StableBalance { oracle_addr: Addr },
}

/// The execute message variants each Lpp must implement
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub enum ExecuteMsg {
    DistributeRewards(),
}
