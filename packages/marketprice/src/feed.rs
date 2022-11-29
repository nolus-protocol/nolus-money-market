use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use finance::duration::Duration;
use sdk::{
    cosmwasm_std::{Addr, Timestamp},
    schemars::{self, JsonSchema},
};

use crate::{error::PriceFeedsError, market_price::Config, SpotPrice};

#[derive(Serialize, Deserialize, Default, PartialEq, Eq, JsonSchema)]
pub struct PriceFeed {
    observations: Vec<Observation>,
}

impl PriceFeed {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_observation(
        mut self,
        from: Addr,
        at: Timestamp,
        price: SpotPrice,
        validity: Duration,
    ) -> Self {
        self.observations.retain(valid_observations(at, validity));

        self.observations.push(Observation::new(from, at, price));
        self
    }

    // provide no price for a pair if there are no observations from at least configurable percentage * <number_of_whitelisted_feeders>
    // in a configurable period T in seconds
    // provide the last price for a requested pair unless the previous condition is met.
    pub fn get_price(&self, config: Config) -> Result<SpotPrice, PriceFeedsError> {
        let last_observation = self
            .valid_observations(&config)
            .last()
            .ok_or(PriceFeedsError::NoPrice {})?;

        if !self.has_enough_feeders(&config) {
            return Err(PriceFeedsError::NoPrice {});
        }

        Ok(last_observation.price.clone())
    }

    fn has_enough_feeders(&self, config: &Config) -> bool {
        self.count_unique_feeders(config) >= config.feeders()
    }

    fn count_unique_feeders(&self, config: &Config) -> usize {
        self.valid_observations(config)
            .map(|o| &o.feeder_addr)
            .collect::<HashSet<_>>()
            .len()
    }

    fn valid_observations(&self, config: &Config) -> impl Iterator<Item = &Observation> {
        let mut valid_observations = valid_observations(config.block_time(), config.period());
        self.observations
            .iter()
            .filter(move |&o| valid_observations(o))
    }
}

fn valid_observations(at: Timestamp, period: Duration) -> impl FnMut(&Observation) -> bool {
    move |o: &Observation| o.valid(at, period)
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
struct Observation {
    feeder_addr: Addr,
    time: Timestamp,
    price: SpotPrice,
}
impl Observation {
    fn new(feeder_addr: Addr, time: Timestamp, price: SpotPrice) -> Observation {
        Observation {
            feeder_addr,
            time,
            price,
        }
    }

    fn valid(&self, at: Timestamp, validity: Duration) -> bool {
        self.time + validity > at
    }
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

    use crate::{error::PriceFeedsError, market_price::Config, SpotPrice};

    use super::PriceFeed;

    #[test]
    fn old_observations() {
        const ONE_FEEDER: usize = 1;
        let validity_period = Duration::from_secs(60);
        let block_time = Timestamp::from_seconds(100);
        let config = Config::new(validity_period, ONE_FEEDER, block_time);

        let feeder1 = Addr::unchecked("feeder1");
        let feed1_time = block_time - validity_period;
        let feed1_price: SpotPrice = price::total_of(Coin::<Weth>::new(20))
            .is(Coin::<Usdc>::new(5000))
            .into();

        let mut feed = PriceFeed::new();
        feed = feed.add_observation(feeder1.clone(), feed1_time, feed1_price, config.period());

        assert_eq!(Err(PriceFeedsError::NoPrice()), feed.get_price(config));

        let feed2_time = feed1_time + Duration::from_nanos(1);
        let feed2_price: SpotPrice = price::total_of(Coin::<Weth>::new(19))
            .is(Coin::<Usdc>::new(5000))
            .into();
        feed = feed.add_observation(
            feeder1,
            feed2_time,
            feed2_price.clone(),
            Duration::from_nanos(0),
        );
        assert_eq!(Ok(feed2_price), feed.get_price(config));
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

        let mut feed = PriceFeed::new();
        feed = feed.add_observation(feeder1, feed1_time, feed1_price.clone(), validity_period);

        let config_two_feeders = Config::new(validity_period, 2, block_time);
        assert_eq!(
            Err(PriceFeedsError::NoPrice()),
            feed.get_price(config_two_feeders)
        );

        let config_one_feeder = Config::new(validity_period, 1, block_time);
        assert_eq!(Ok(feed1_price), feed.get_price(config_one_feeder));
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

        let mut feed = PriceFeed::new();
        feed = feed.add_observation(feeder1, feed1_time, feed1_price, validity_period);

        let feeder2 = Addr::unchecked("feeder2");
        let feed2_time = block_time - validity_period + Duration::from_nanos(1);
        let feed2_price: SpotPrice = price::total_of(Coin::<Weth>::new(19))
            .is(Coin::<Usdc>::new(5000))
            .into();
        feed = feed.add_observation(feeder2, feed2_time, feed2_price, validity_period);

        let config_feed1_and_2_in = Config::new(validity_period, 2, feed2_time);
        assert!(feed.get_price(config_feed1_and_2_in).is_ok());

        let config_feed2_in = Config::new(validity_period, 2, block_time);
        assert_eq!(
            Err(PriceFeedsError::NoPrice()),
            feed.get_price(config_feed2_in)
        );
    }
}
