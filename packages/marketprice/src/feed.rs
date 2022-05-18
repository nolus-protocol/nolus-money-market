use std::collections::HashSet;

use cosmwasm_std::{Addr, Decimal256, Timestamp};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::market_price::PriceFeedsError;

pub type Denom = String;
pub type DenomPair = (Denom, Denom);

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Price {
    pub amount: Decimal256,
    pub denom: Denom,
}

impl Price {
    pub fn new(amount: Decimal256, denom: Denom) -> Self {
        Price { amount, denom }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Prices {
    pub base: Denom,
    pub values: Vec<Price>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct DenomToPrice {
    pub denom: Denom,
    pub price: Price,
}
impl DenomToPrice {
    pub fn new(denom: Denom, price: Price) -> Self {
        DenomToPrice { denom, price }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Observation {
    feeder_addr: Addr,
    time: Timestamp,
    price: Decimal256,
}
impl Observation {
    pub fn new(feeder_addr: Addr, time: Timestamp, price: Decimal256) -> Observation {
        Observation {
            feeder_addr,
            time,
            price,
        }
    }
    pub fn price(&self) -> Decimal256 {
        self.price
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct PriceFeed {
    observations: Vec<Observation>,
}

impl PriceFeed {
    pub fn new(new_feed: Observation) -> PriceFeed {
        PriceFeed {
            observations: vec![new_feed],
        }
    }

    pub fn update(&mut self, new_feed: Observation, price_feed_period: u64) {
        // drop all feeds older than the required refresh time
        self.observations
            .retain(|f| !PriceFeed::is_old_feed(new_feed.time, f.time, price_feed_period));

        self.observations.push(new_feed);
    }

    // provide no price for a pair if there are no feeds from at least configurable percentage * <number_of_whitelisted_feeders>
    // in a configurable period T in seconds
    // provide the last price for a requested pair unless the previous condition is met.
    pub fn get_price(
        &self,
        time_now: Timestamp,
        price_feed_period: u64,
        required_feeders_cnt: usize,
    ) -> Result<Observation, PriceFeedsError> {
        let res = self.observations.last().cloned();
        let last_feed = match res {
            Some(f) => f,
            None => return Err(PriceFeedsError::NoPrice {}),
        };

        // check if last reported feed is older than the required refresh time
        if PriceFeed::is_old_feed(time_now, last_feed.time, price_feed_period) {
            return Err(PriceFeedsError::NoPrice {});
        }

        if !self.has_enough_feeders(required_feeders_cnt) {
            return Err(PriceFeedsError::NoPrice {});
        }

        Ok(last_feed)
    }

    fn is_old_feed(time_now: Timestamp, feed_time: Timestamp, price_feed_period: u64) -> bool {
        let ts = feed_time.plus_seconds(price_feed_period);
        ts.lt(&time_now)
    }

    fn has_enough_feeders(&self, required_feeders_cnt: usize) -> bool {
        let unique_reported_feeders = PriceFeed::filter_uniq(&self.observations);
        unique_reported_feeders.len() >= required_feeders_cnt
    }

    fn filter_uniq(vec: &[Observation]) -> HashSet<&Addr> {
        vec.iter().map(|o| &o.feeder_addr).collect::<HashSet<_>>()
    }
}
