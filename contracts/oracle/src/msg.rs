use cosmwasm_std::{Addr, Timestamp};
use marketprice::feed::{Denom, DenomPair, DenomToPrice, Prices};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub base_asset: String,
    pub price_feed_period: u64,
    pub feeders_percentage_needed: u8,
    pub supported_denom_pairs: Vec<DenomPair>,
    pub timealarms_addr: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    RegisterFeeder {
        feeder_address: String,
    },
    FeedPrices {
        prices: Vec<Prices>, // (asset, [(asset1, price), (asset2, price)])
    },
    Config {
        price_feed_period: u64,
        feeders_percentage_needed: u8,
    },
    SupportedDenomPairs {
        pairs: Vec<DenomPair>,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    // returns the contract configuration
    Config {},
    // returns all registered feeders
    Feeders {},
    // check if an address belongs to a registered feeder
    IsFeeder { address: Addr },
    // returns the price of the denom against the base asset
    PriceFor { denoms: Vec<Denom> },
    // returns a list of supported denom pairs
    SupportedDenomPairs {},
}

// We define a custom struct for each query response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ConfigResponse {
    pub base_asset: String,
    pub price_feed_period: u64,
    pub feeders_percentage_needed: u8,
    pub owner: Addr,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct PriceResponse {
    pub prices: Vec<DenomToPrice>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteAlarmMsg {
    Alarm(Timestamp),
}
