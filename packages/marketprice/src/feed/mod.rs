use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use finance::{
    currency::Currency, duration::Duration, fraction::Fraction, percent::Percent, price::Price,
};
use sdk::cosmwasm_std::{Addr, Timestamp};

use crate::{error::PriceFeedsError, feed::sample::Sample, market_price::Config};

use self::observation::Observation;

mod observation;
mod sample;

#[derive(Serialize, Deserialize, Default)]
pub struct PriceFeed<C, QuoteC>
where
    C: Currency,
    QuoteC: Currency,
{
    observations: Vec<Observation<C, QuoteC>>,
}

impl<C, QuoteC> PriceFeed<C, QuoteC>
where
    C: Currency,
    QuoteC: Currency,
{
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_observation(
        mut self,
        from: Addr,
        at: Timestamp,
        price: Price<C, QuoteC>,
        validity: Duration,
    ) -> Self {
        self.observations
            .retain(observation::valid_at(at, validity));

        self.observations.push(Observation::new(from, at, price));
        self
    }

    /// Calculate the price of this feed
    ///
    /// Provide no price if there are no observations from at least configurable percentage * <number_of_whitelisted_feeders>.
    /// Observations older than a configurable period are not taken into consideration.
    /// Calculate the price at a sample period as per the formula:
    /// discounting_factor * avg_price_during_the_period + (1 - discounting_factor) * price_at_the_previos_period
    pub fn calc_price(&self, config: &Config) -> Result<Price<C, QuoteC>, PriceFeedsError> {
        if !self.has_enough_feeders(config) {
            return Err(PriceFeedsError::NoPrice {});
        }

        // TODO: move to config
        const SAMPLE_PERIOD: Duration = Duration::from_secs(5);
        let discount_factor = Percent::from_percent(75);
        assert!(discount_factor < Percent::HUNDRED);
        assert!(SAMPLE_PERIOD < config.period());

        let observations = self.valid_observations(config);
        let validity_period = config.block_time() - config.period();

        let samples = sample::from_observations(observations, validity_period, SAMPLE_PERIOD);

        samples
            .take((config.period().nanos() / SAMPLE_PERIOD.nanos()).try_into()?)
            .map(Sample::into_maybe_price)
            .skip_while(Option::is_none)
            .map(|price| Option::expect(price, "sample prices should keep being present"))
            .reduce(|acc, sample_price| {
                discount_factor.of(sample_price) + (Percent::HUNDRED - discount_factor).of(acc)
            })
            .ok_or(PriceFeedsError::NoPrice {})
    }

    fn has_enough_feeders(&self, config: &Config) -> bool {
        self.count_unique_feeders(config) >= config.feeders()
    }

    fn count_unique_feeders(&self, config: &Config) -> usize {
        self.valid_observations(config)
            .map(Observation::feeder)
            .collect::<HashSet<_>>()
            .len()
    }

    fn valid_observations(&self, config: &Config) -> impl Iterator<Item = &Observation<C, QuoteC>> {
        let mut valid_observations = observation::valid_at(config.block_time(), config.period());
        self.observations
            .iter()
            .filter(move |&o| valid_observations(o))
    }
}

#[cfg(test)]
mod test {
    use currency::{lease::Weth, lpn::Usdc};
    use finance::{
        coin::{Amount, Coin},
        duration::Duration,
        price::{self, Price},
    };
    use sdk::cosmwasm_std::{Addr, Timestamp};

    use crate::{error::PriceFeedsError, market_price::Config};

    use super::PriceFeed;

    #[test]
    fn old_observations() {
        const ONE_FEEDER: usize = 1;
        let validity_period = Duration::from_secs(60);
        let block_time = Timestamp::from_seconds(100);
        let config = Config::new(validity_period, ONE_FEEDER, block_time);

        let feeder1 = Addr::unchecked("feeder1");
        let feed1_time = block_time - validity_period;
        let feed1_price = price(20, 5000);

        let mut feed = PriceFeed::new();
        feed = feed.add_observation(feeder1.clone(), feed1_time, feed1_price, config.period());

        assert_eq!(Err(PriceFeedsError::NoPrice()), feed.calc_price(&config));

        let feed2_time = feed1_time + Duration::from_nanos(1);
        let feed2_price = price(19, 5000);
        feed = feed.add_observation(feeder1, feed2_time, feed2_price, Duration::from_nanos(0));
        assert_eq!(Ok(feed2_price), feed.calc_price(&config));
    }

    #[test]
    fn less_feeders() {
        let validity_period = Duration::from_secs(60);
        let block_time = Timestamp::from_seconds(100);

        let feeder1 = Addr::unchecked("feeder1");
        let feed1_time = block_time;
        let feed1_price = price(20, 5000);

        let mut feed = PriceFeed::new();
        feed = feed.add_observation(feeder1, feed1_time, feed1_price, validity_period);

        let config_two_feeders = Config::new(validity_period, 2, block_time);
        assert_eq!(
            Err(PriceFeedsError::NoPrice()),
            feed.calc_price(&config_two_feeders)
        );

        let config_one_feeder = Config::new(validity_period, 1, block_time);
        assert_eq!(Ok(feed1_price), feed.calc_price(&config_one_feeder));
    }

    #[test]
    fn less_feeders_with_valid_observations() {
        let validity_period = Duration::from_secs(60);
        let block_time = Timestamp::from_seconds(150);

        let feeder1 = Addr::unchecked("feeder1");
        let feed1_time = block_time - validity_period;
        let feed1_price = price(20, 5000);

        let mut feed = PriceFeed::new();
        feed = feed.add_observation(feeder1, feed1_time, feed1_price, validity_period);

        let feeder2 = Addr::unchecked("feeder2");
        let feed2_time = block_time - validity_period + Duration::from_nanos(1);
        let feed2_price = price(19, 5000);
        feed = feed.add_observation(feeder2, feed2_time, feed2_price, validity_period);

        let config_feed1_and_2_in = Config::new(validity_period, 2, feed2_time);
        assert!(feed.calc_price(&config_feed1_and_2_in).is_ok());

        let config_feed2_in = Config::new(validity_period, 2, block_time);
        assert_eq!(
            Err(PriceFeedsError::NoPrice()),
            feed.calc_price(&config_feed2_in)
        );
    }

    #[test]
    fn ema_price() {
        let validity_period = Duration::from_secs(60);
        let block_time = Timestamp::from_seconds(100);
        let config = Config::new(validity_period, 1, block_time);

        let s1 = block_time - Duration::from_secs(12);
        let s21 = block_time - Duration::from_secs(7);
        let s22 = block_time - Duration::from_secs(6);
        let s3 = block_time - Duration::from_secs(2);

        let feeder1 = Addr::unchecked("feeder1");
        let feeder2 = Addr::unchecked("feeder2");

        let mut feed = PriceFeed::new();
        feed = feed.add_observation(feeder1.clone(), s1, price(19, 5160), config.period());
        feed = feed.add_observation(feeder1.clone(), s21, price(19, 5500), config.period());
        feed = feed.add_observation(feeder1.clone(), s22, price(19, 5000 + 10), config.period());
        feed = feed.add_observation(feeder2, s22, price(19, 5000 - 10), config.period());
        feed = feed.add_observation(feeder1, s3, price(19, 5000), config.period());

        assert_eq!(Ok(price(19, 5010)), feed.calc_price(&config));
    }

    fn price(c: Amount, q: Amount) -> Price<Weth, Usdc> {
        price::total_of(Coin::from(c)).is(Coin::from(q))
    }
}
