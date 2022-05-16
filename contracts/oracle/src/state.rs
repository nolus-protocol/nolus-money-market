use cw_storage_plus::Item;
use marketprice::{feed::DenomPair, feeders::PriceFeeders, market_price::PriceFeeds};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::Addr;
use time_oracle::{TimeOracle, Alarms};

pub const CONFIG: Item<Config> = Item::new("config");
pub const FEEDERS: PriceFeeders = PriceFeeders::new("feeders");
pub const MARKET_PRICE: PriceFeeds = PriceFeeds::new("market_price");
pub const TIME_ORACLE: TimeOracle = TimeOracle::new("time_oracle");
pub const TIME_ALARMS: Alarms = Alarms::new("alarms", "alarms_idx", "alarms_next_id");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub base_asset: String,
    pub owner: Addr,
    pub price_feed_period: u64,
    pub feeders_percentage_needed: u8,
    pub supported_denom_pairs: Vec<DenomPair>,
}
