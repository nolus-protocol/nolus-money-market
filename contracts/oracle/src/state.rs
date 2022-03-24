use cw_storage_plus::Item;
use marketprice::{market_price::PriceFeeds, feeders::PriceFeeders};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::Addr;
use time_oracle::TimeOracle;


pub const CONFIG: Item<Config> = Item::new("config");
pub const FEEDERS: PriceFeeders = PriceFeeders::new("feeders");
pub const MARKET_PRICE: PriceFeeds = PriceFeeds::new("market_price");
pub const TIME_ORACLE: TimeOracle = TimeOracle::new("namespace");


#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub base_asset: String,
    pub owner: Addr,
    pub price_feed_period: u64,
    pub feeders_percentage_needed: u8,
}