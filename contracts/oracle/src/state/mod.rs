use serde::{Deserialize, Serialize};

use finance::{currency::SymbolOwned, duration::Duration, percent::Percent};
use sdk::schemars::{self, JsonSchema};

pub mod config;
pub mod supported_pairs;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct Config {
    pub base_asset: SymbolOwned,
    pub price_feed_period: Duration,
    pub expected_feeders: Percent,
}
