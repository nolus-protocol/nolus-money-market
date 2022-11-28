use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use finance::duration::Duration;
use sdk::{
    cosmwasm_std::{Addr, Timestamp},
    schemars::{self, JsonSchema},
};

use crate::{error::PriceFeedsError, market_price::Parameters, SpotPrice};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct Observation {
    feeder_addr: Addr,
    time: Timestamp,
    price: SpotPrice,
}
impl Observation {
    pub fn new(feeder_addr: Addr, time: Timestamp, price: SpotPrice) -> Observation {
        Observation {
            feeder_addr,
            time,
            price,
        }
    }
    pub fn price(&self) -> SpotPrice {
        self.price.clone()
    }

    pub fn valid(&self, at: Timestamp, validity: Duration) -> bool {
        self.time + validity > at
    }
}

#[derive(Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
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
            .retain(valid_observations(new_feed.time, price_feed_period));

        self.observations.push(new_feed);
    }

    // provide no price for a pair if there are no feeds from at least configurable percentage * <number_of_whitelisted_feeders>
    // in a configurable period T in seconds
    // provide the last price for a requested pair unless the previous condition is met.
    pub fn get_price(&self, parameters: Parameters) -> Result<SpotPrice, PriceFeedsError> {
        let last_feed = self
            .observations
            .last()
            .ok_or(PriceFeedsError::NoPrice {})?;

        if !last_feed.valid(parameters.block_time(), parameters.period()) {
            return Err(PriceFeedsError::NoPrice {});
        }

        if !self.has_enough_feeders(&parameters) {
            return Err(PriceFeedsError::NoPrice {});
        }

        Ok(last_feed.price.clone())
    }

    fn has_enough_feeders(&self, params: &Parameters) -> bool {
        self.count_unique_feeders(params) >= params.feeders()
    }

    fn count_unique_feeders(&self, params: &Parameters) -> usize {
        let mut valid_observations = valid_observations(params.block_time(), params.period());
        self.observations
            .iter()
            .filter(|&o| valid_observations(o))
            .map(|o| &o.feeder_addr)
            .collect::<HashSet<_>>()
            .len()
    }
}

fn valid_observations(at: Timestamp, period: Duration) -> impl FnMut(&Observation) -> bool {
    move |o: &Observation| o.valid(at, period)
}

#[cfg(test)]
mod test {
    use currency::{lease::Weth, lpn::Usdc};
    use finance::{
        coin::Coin,
        duration::Duration,
        price::{self},
    };
    use sdk::cosmwasm_std::{Addr, Timestamp};

    use crate::{error::PriceFeedsError, market_price::Parameters, SpotPrice};

    use super::{Observation, PriceFeed};

    #[test]
    fn old_observations() {
        const ONE_FEEDER: usize = 1;
        let validity_period = Duration::from_secs(60);
        let block_time = Timestamp::from_seconds(100);
        let params = Parameters::new(validity_period, ONE_FEEDER, block_time);

        let feeder1 = Addr::unchecked("feeder1");
        let feed1_time = block_time - validity_period;
        let feed1_price: SpotPrice = price::total_of(Coin::<Weth>::new(20))
            .is(Coin::<Usdc>::new(5000))
            .into();

        let mut feed = PriceFeed::new(Observation::new(feeder1.clone(), feed1_time, feed1_price));

        assert_eq!(Err(PriceFeedsError::NoPrice()), feed.get_price(params));

        let feed2_time = feed1_time + Duration::from_nanos(1);
        let feed2_price: SpotPrice = price::total_of(Coin::<Weth>::new(19))
            .is(Coin::<Usdc>::new(5000))
            .into();
        feed.update(
            Observation::new(feeder1, feed2_time, feed2_price.clone()),
            Duration::from_nanos(0),
        );
        assert_eq!(Ok(feed2_price), feed.get_price(params));
    }

    #[test]
    fn less_feeders() {
        let validity_period = Duration::from_secs(60);
        let block_time = Timestamp::from_seconds(100);

        let feeder1 = Addr::unchecked("feeder1");
        let feed1_time = block_time;
        let feed1_price: SpotPrice = price::total_of(Coin::<Weth>::new(20))
            .is(Coin::<Usdc>::new(5000))
            .into();

        let feed = PriceFeed::new(Observation::new(feeder1, feed1_time, feed1_price.clone()));

        let params_two_feeders = Parameters::new(validity_period, 2, block_time);
        assert_eq!(
            Err(PriceFeedsError::NoPrice()),
            feed.get_price(params_two_feeders)
        );

        let params_one_feeder = Parameters::new(validity_period, 1, block_time);
        assert_eq!(Ok(feed1_price), feed.get_price(params_one_feeder));
    }

    #[test]
    fn less_feeders_with_valid_observations() {
        let validity_period = Duration::from_secs(60);
        let block_time = Timestamp::from_seconds(100);

        let feeder1 = Addr::unchecked("feeder1");
        let feed1_time = block_time - validity_period;
        let feed1_price: SpotPrice = price::total_of(Coin::<Weth>::new(20))
            .is(Coin::<Usdc>::new(5000))
            .into();

        let mut feed = PriceFeed::new(Observation::new(feeder1, feed1_time, feed1_price));

        let feeder2 = Addr::unchecked("feeder2");
        let feed2_time = block_time - validity_period + Duration::from_nanos(1);
        let feed2_price: SpotPrice = price::total_of(Coin::<Weth>::new(19))
            .is(Coin::<Usdc>::new(5000))
            .into();
        feed.update(
            Observation::new(feeder2, feed2_time, feed2_price),
            validity_period,
        );

        let params_feed1_and_2_in = Parameters::new(validity_period, 2, feed2_time);
        assert!(feed.get_price(params_feed1_and_2_in).is_ok());

        let params_feed2_in = Parameters::new(validity_period, 2, block_time);
        assert_eq!(
            Err(PriceFeedsError::NoPrice()),
            feed.get_price(params_feed2_in)
        );
    }
}
