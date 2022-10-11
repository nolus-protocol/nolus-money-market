use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use finance::{currency::SymbolOwned, duration::Duration, percent::Percent, price::dto::PriceDTO};
use marketprice::alarms::Alarm;
use sdk::{
    cosmwasm_std::Addr,
    schemars::{self, JsonSchema},
};

use crate::state::supported_pairs::ResolutionPath;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct InstantiateMsg {
    pub base_asset: String,
    pub price_feed_period_secs: u32,
    pub expected_feeders: Percent,
    pub currency_paths: Vec<ResolutionPath>,
    pub timealarms_addr: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    RegisterFeeder {
        feeder_address: String,
    },
    RemoveFeeder {
        feeder_address: String,
    },
    FeedPrices {
        prices: Vec<PriceDTO>,
    },
    Config {
        price_feed_period_secs: u32,
        expected_feeders: Percent,
    },
    CurrencyPaths {
        paths: Vec<ResolutionPath>,
    },
    AddPriceAlarm {
        alarm: Alarm,
    },
    RemovePriceAlarm {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    // returns the contract configuration
    Config {},
    // returns all registered feeders
    Feeders {},
    // check if an address belongs to a registered feeder
    IsFeeder { address: Addr },
    // returns the price of the denom against the base asset
    Prices { currencies: HashSet<SymbolOwned> },
    Price { currency: SymbolOwned },
    // returns a list of supported denom pairs
    SupportedDenomPairs {},
}

// We define a custom struct for each query response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct ConfigResponse {
    pub base_asset: SymbolOwned,
    pub price_feed_period: Duration,
    pub expected_feeders: Percent,
    pub owner: Addr,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct PricesResponse {
    pub prices: Vec<PriceDTO>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteAlarmMsg {
    PriceAlarm(),
}
