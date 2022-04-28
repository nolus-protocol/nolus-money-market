use cosmwasm_std::{Addr, Decimal256, Timestamp};
use marketprice::feed::Denom;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub base_asset: String,
    pub price_feed_period: u64,
    pub feeders_percentage_needed: u8
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    RegisterFeeder { feeder_address: String },
    FeedPrice {
        base: Denom,
        prices: Vec<(Denom, Decimal256)>, // (asset, price)
    },
    Config {
        price_feed_period: u64,
        feeders_percentage_needed: u8
    },
    AddAlarm {
        addr: Addr,
        time: Timestamp,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    Config {},
    Feeders {},
    IsFeeder { address: Addr },
    Price {
        base: Denom,
        quote: Denom,
    }
}

// We define a custom struct for each query response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ConfigResponse {
    pub base_asset: String,
    pub owner: Addr,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteAlarmMsg {
    Alarm(Timestamp),
}
