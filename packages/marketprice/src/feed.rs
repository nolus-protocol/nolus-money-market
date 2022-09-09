use std::collections::HashSet;

use cosmwasm_std::{Addr, Timestamp};
use finance::price::PriceDTO;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::market_price::{Parameters, PriceFeedsError};
use finance::duration::Duration;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct Observation {
    feeder_addr: Addr,
    time: Timestamp,
    price: PriceDTO,
}
impl Observation {
    pub fn new(feeder_addr: Addr, time: Timestamp, price: PriceDTO) -> Observation {
        Observation {
            feeder_addr,
            time,
            price,
        }
    }
    pub fn price(&self) -> PriceDTO {
        self.price.clone()
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
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
    pub fn get_price(&self, parameters: Parameters) -> Result<Observation, PriceFeedsError> {
        let res = self.observations.last().cloned();
        let last_feed = match res {
            Some(f) => f,
            None => return Err(PriceFeedsError::NoPrice {}),
        };

        // check if last reported feed is older than the required refresh time
        if PriceFeed::is_old_feed(parameters.block_time(), last_feed.time, parameters.period()) {
            return Err(PriceFeedsError::NoPrice {});
        }

        if !self.has_enough_feeders(parameters.feeders()) {
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
    use finance::{
        coin::Coin,
        currency::{Currency, Nls, SymbolStatic},
        price::{self, PriceDTO},
    };

    #[test]
    // we ensure this rounds up (as it calculates needed votes)
    fn compare_prices() {
        #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default)]
        pub struct BTH;
        impl Currency for BTH {
            const SYMBOL: SymbolStatic = "BTH";
        }
        #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default)]
        pub struct ETH;
        impl Currency for ETH {
            const SYMBOL: SymbolStatic = "ETH";
        }

        let p1 = PriceDTO::try_from(
            price::total_of(Coin::<BTH>::new(1000000)).is(Coin::<Nls>::new(123456)),
        )
        .unwrap();
        let p2 = PriceDTO::try_from(
            price::total_of(Coin::<BTH>::new(1000000)).is(Coin::<Nls>::new(789456)),
        )
        .unwrap();
        let p3 = PriceDTO::try_from(
            price::total_of(Coin::<BTH>::new(1000000)).is(Coin::<Nls>::new(3456)),
        )
        .unwrap();
        let p4 = PriceDTO::try_from(
            price::total_of(Coin::<ETH>::new(1000000)).is(Coin::<Nls>::new(3456)),
        )
        .unwrap();
        assert!(p1.lt(&p2));
        assert!(p3.lt(&p2));
        assert!(p4.lt(&p2));
    }
}
