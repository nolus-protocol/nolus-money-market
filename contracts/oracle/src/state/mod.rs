use serde::{Deserialize, Serialize};

use finance::{currency::SymbolOwned, duration::Duration, percent::Percent};
use sdk::{
    cosmwasm_std::Addr,
    schemars::{self, JsonSchema},
};

pub mod config;
pub mod supported_pairs;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct Config {
    pub base_asset: SymbolOwned,
    pub owner: Addr,
    pub price_feed_period: Duration,
    pub expected_feeders: Percent,
    pub timealarms_contract: Addr,
}
