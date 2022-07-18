use std::collections::HashSet;

use cosmwasm_std::{Addr, Decimal, Timestamp};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::market_price::PriceFeedsError;
use finance::duration::Duration;

pub type Denom = String;
pub type DenomPair = (Denom, Denom);

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Price {
    pub amount: Decimal,
    pub denom: Denom,
}

impl Price {
    pub fn new(amount: Decimal, denom: Denom) -> Self {
        Price { amount, denom }
    }

    pub fn is_below(&self, target: &Price) -> bool {
        self.denom.eq(&target.denom) && self.amount.lt(&target.amount)
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

    pub fn is_below(&self, target: &DenomToPrice) -> bool {
        self.denom.eq(&target.denom) && self.price.is_below(&target.price)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Observation {
    feeder_addr: Addr,
    time: Timestamp,
    price: Decimal,
}
impl Observation {
    pub fn new(feeder_addr: Addr, time: Timestamp, price: Decimal) -> Observation {
        Observation {
            feeder_addr,
            time,
            price,
        }
    }
    pub fn price(&self) -> Decimal {
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

    pub fn update(&mut self, new_feed: Observation, price_feed_period: Duration) {
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
        price_feed_period: Duration,
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

    fn is_old_feed(time_now: Timestamp, feed_time: Timestamp, price_feed_period: Duration) -> bool {
        let ts = feed_time + price_feed_period;
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

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use cosmwasm_std::Decimal;

    use crate::feed::{DenomToPrice, Price};

    #[test]
    // we ensure this rounds up (as it calculates needed votes)
    fn compare_prices() {
        let p1 = DenomToPrice::new(
            "BTH".to_string(),
            Price::new(Decimal::from_str("0.123456").unwrap(), "NLS".to_string()),
        );

        let p2 = DenomToPrice::new(
            "BTH".to_string(),
            Price::new(Decimal::from_str("0.789456").unwrap(), "NLS".to_string()),
        );

        let p3 = DenomToPrice::new(
            "BTH".to_string(),
            Price::new(Decimal::from_str("0.003456").unwrap(), "NLS".to_string()),
        );

        let p4 = DenomToPrice::new(
            "ETH".to_string(),
            Price::new(Decimal::from_str("0.003456").unwrap(), "NLS".to_string()),
        );

        assert!(p1.is_below(&p2));
        assert!(p3.is_below(&p2));
        assert!(!p4.is_below(&p2));
    }
}
