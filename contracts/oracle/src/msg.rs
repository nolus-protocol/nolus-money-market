use std::collections::HashSet;

use cosmwasm_std::Addr;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use finance::currency::SymbolOwned;
use finance::price::PriceDTO;
use marketprice::{
    alarms::Alarm,
    storage::{Denom, DenomPair, Price},
};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct InstantiateMsg {
    pub base_asset: String,
    pub price_feed_period_secs: u32,
    pub feeders_percentage_needed: u8,
    pub supported_denom_pairs: Vec<DenomPair>,
    pub timealarms_addr: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    RegisterFeeder {
        feeder_address: String,
    },
    FeedPrices {
        prices: Vec<Price>,
    },
    Config {
        price_feed_period_secs: u32,
        feeders_percentage_needed: u8,
    },
    SupportedDenomPairs {
        pairs: Vec<DenomPair>,
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
    PriceFor { denoms: HashSet<Denom> },
    Price { denom: Denom },
    // returns a list of supported denom pairs
    SupportedDenomPairs {},
}

// We define a custom struct for each query response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct ConfigResponse {
    pub base_asset: SymbolOwned,
    pub price_feed_period_secs: u32,
    pub feeders_percentage_needed: u8,
    pub owner: Addr,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct PricesResponse {
    pub prices: Vec<Price>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct PriceResponse {
    pub price: PriceDTO,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteAlarmMsg {
    PriceAlarm(),
}
