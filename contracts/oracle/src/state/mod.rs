use cosmwasm_std::Addr;
use finance::{currency::SymbolOwned, duration::Duration, percent::Percent};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub mod config;
pub mod supported_pairs;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct Config {
    pub base_asset: SymbolOwned,
    pub owner: Addr,
    pub price_feed_period: Duration,
    pub feeders_percentage_needed: Percent,
    pub timealarms_contract: Addr,
}
