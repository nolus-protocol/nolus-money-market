use std::{collections::HashSet, marker::PhantomData};

use finance::{error::Error as FinanceError, price::Price};
use observations::{Observations, ObservationsRead};
use sdk::cosmwasm_std::{Addr, Timestamp};

use crate::{
    config::Config,
    error::{PriceFeedsError, Result},
    feed::sample::Sample,
    feeders::Count,
};

pub(crate) use self::observation::Observation;
pub use self::{
    cw::Repo,
    observations::{ObservationsReadRepo, ObservationsRepo},
};

mod cw;
#[cfg(test)]
mod memory;
mod observation;
mod observations;
mod sample;

pub struct PriceFeed<C, QuoteC, ObservationsImpl>
where
    C: 'static,
    QuoteC: 'static,
{
    observations: ObservationsImpl,
    _c_type: PhantomData<C>,
    _quote_c_type: PhantomData<QuoteC>,
}

impl<C, QuoteC, ObservationsImpl> PriceFeed<C, QuoteC, ObservationsImpl>
where
    C: 'static,
    QuoteC: 'static,
{
    pub fn with(observations: ObservationsImpl) -> Self {
        Self {
            observations,
            _c_type: PhantomData,
            _quote_c_type: PhantomData,
        }
    }
}

impl<C, QuoteC, ObservationsImpl> PriceFeed<C, QuoteC, ObservationsImpl>
where
    C: 'static,
    QuoteC: 'static,
    ObservationsImpl: ObservationsRead<C = C, QuoteC = QuoteC>,
{
    /// Calculate the price of this feed
    ///
    /// Provide no price if there are no observations from at least configurable percentage * <number_of_whitelisted_feeders>.
    /// Observations older than a configurable period are not taken into consideration.
    /// Calculate the price at a sample period as per the formula:
    /// discounting_factor * avg_price_during_the_period + (1 - discounting_factor) * price_at_the_previos_period
    pub fn calc_price(
        &self,
        config: &Config,
        at: Timestamp,
        total_feeders: Count,
    ) -> Result<Price<C, QuoteC>> {
        let valid_since = config.feed_valid_since(at);
        // a trade-off of eager loading of the observations from the persistence
        // vs. lazy-loading twice - checking the total number of unique feeders and samples generation
        let observations = self.valid_observations(&valid_since)?;

        if !self.has_enough_feeders(observations.iter(), config, total_feeders) {
            return Err(PriceFeedsError::NoPrice {});
        }

        let samples =
            sample::from_observations(observations.iter(), valid_since, config.sample_period());

        let discount_factor = config.discount_factor();

        let samples_nb = config.samples_number().into();

        let mut item_iter = samples
            .take(samples_nb)
            .map(Sample::into_maybe_price)
            .skip_while(Option::is_none)
            .map(|price| Option::expect(price, "sample prices should keep being present"));

        let first = item_iter.next().ok_or(PriceFeedsError::NoPrice {})?;

        item_iter
            .try_fold(first, |acc, current| {
                current.lossy_mul::<_, u128>(discount_factor).and_then(|a| {
                    acc.lossy_mul::<_, u128>(discount_factor.complement())
                        .and_then(|b| a.checked_add(b))
                })
            })
            .ok_or(PriceFeedsError::Finance(FinanceError::Overflow(
                "Overflow while calculating the sum of the prices",
            )))

        // samples
        //     .take(samples_nb)
        //     .map(Sample::into_maybe_price)
        //     .skip_while(Option::is_none)
        //     .map(|price| Option::expect(price, "sample prices should keep being present"))
        //     .reduce(|acc, sample_price| {
        //         sample_price.lossy_mul::<_, u128>(discount_factor)
        //             + acc.lossy_mul::<_, u128>(discount_factor.complement())
        //     })
        //     .ok_or(PriceFeedsError::NoPrice {})
    }

    fn valid_observations(&self, since: &Timestamp) -> Result<Vec<Observation<C, QuoteC>>> {
        self.observations.as_iter().and_then(|mut items| {
            items.try_fold(
                Vec::with_capacity(self.observations.len()),
                |mut acc, may_item| {
                    may_item.map(|item| {
                        if item.valid_since(since) {
                            acc.push(item);
                        }
                        acc
                    })
                },
            )
        })
    }

    fn has_enough_feeders<'items, Observations>(
        &self,
        items: Observations,
        config: &Config,
        total_feeders: Count,
    ) -> bool
    where
        Observations: for<'item> Iterator<Item = &'items Observation<C, QuoteC>>,
    {
        self.count_unique_feeders(items) >= config.min_feeders(total_feeders)
    }

    fn count_unique_feeders<'items, Observations>(&self, items: Observations) -> Count
    where
        Observations: for<'item> Iterator<Item = &'items Observation<C, QuoteC>>,
    {
        items
            .map(Observation::feeder)
            .collect::<HashSet<_>>()
            .len()
            .try_into()
            .expect("count should fit within defined bounds")
    }
}

impl<C, QuoteC, ObservationsImpl> PriceFeed<C, QuoteC, ObservationsImpl>
where
    C: 'static,
    QuoteC: 'static,
    ObservationsImpl: Observations<C = C, QuoteC = QuoteC>,
{
    pub fn add_observation(
        mut self,
        from: Addr,
        at: Timestamp,
        price: Price<C, QuoteC>,
        valid_since: &Timestamp,
    ) -> Result<Self> {
        debug_assert!(valid_since < &at, "{valid_since} >= {at}");
        self.observations
            .retain(valid_since)
            .and_then(|()| {
                self.observations
                    .register(Observation::new(from, at, price))
            })
            .map(|()| self)
    }
}

#[cfg(test)]
mod test {
    use currency::test::{SuperGroupTestC4, SuperGroupTestC5};
    use finance::{
        coin::{Amount, Coin},
        duration::Duration,
        percent::Percent100,
        price::{self, Price},
    };
    use sdk::cosmwasm_std::{Addr, Timestamp};

    use crate::{config::Config, error::PriceFeedsError, feeders::Count};

    use super::{PriceFeed, memory::InMemoryObservations, observations::Observations};

    const ONE_FEEDER: Count = Count::new_test(1);
    const TWO_FEEDERS: Count = Count::new_test(2);
    const SAMPLE_PERIOD: Duration = Duration::from_secs(5);
    const SAMPLES_NUMBER: u16 = 12;
    const VALIDITY: Duration = Duration::from_secs(60);
    const DISCOUNTING_FACTOR: Percent100 = Percent100::from_permille(750);

    type TestC = SuperGroupTestC4;
    type TestQuoteC = SuperGroupTestC5;

    fn feed() -> PriceFeed<TestC, TestQuoteC, impl Observations<C = TestC, QuoteC = TestQuoteC>> {
        PriceFeed::with(InMemoryObservations::<TestC, TestQuoteC>::new())
    }

    #[test]
    fn old_observations() {
        let block_time = Timestamp::from_seconds(100);
        let config = Config::new(
            Percent100::HUNDRED,
            SAMPLE_PERIOD,
            SAMPLES_NUMBER,
            DISCOUNTING_FACTOR,
        );

        let feeder1 = Addr::unchecked("feeder1");
        let feed1_time = block_time - VALIDITY;
        let feed1_price = price(20, 5000);

        let mut feed = feed();
        feed = feed
            .add_observation(
                feeder1.clone(),
                feed1_time,
                feed1_price,
                &config.feed_valid_since(feed1_time),
            )
            .unwrap();

        assert_eq!(
            Err(PriceFeedsError::NoPrice()),
            feed.calc_price(&config, block_time, ONE_FEEDER)
        );

        let feed2_time = feed1_time + Duration::from_nanos(1);
        let feed2_price = price(19, 5000);
        feed = feed
            .add_observation(feeder1, feed2_time, feed2_price, &feed1_time)
            .unwrap();
        assert_eq!(
            Ok(feed2_price),
            feed.calc_price(&config, block_time, ONE_FEEDER)
        );
    }

    #[test]
    fn less_feeders() {
        let validity_period = Duration::from_secs(60);
        let block_time = Timestamp::from_seconds(100);

        let feeder1 = Addr::unchecked("feeder1");
        let feed1_time = block_time;
        let feed1_price = price(20, 5000);

        let mut feed = feed();
        feed = feed
            .add_observation(
                feeder1,
                feed1_time,
                feed1_price,
                &(block_time - validity_period),
            )
            .unwrap();

        let config = Config::new(
            Percent100::HUNDRED,
            SAMPLE_PERIOD,
            SAMPLES_NUMBER,
            DISCOUNTING_FACTOR,
        );
        assert_eq!(
            Err(PriceFeedsError::NoPrice()),
            feed.calc_price(&config, block_time, TWO_FEEDERS)
        );

        assert_eq!(
            Ok(feed1_price),
            feed.calc_price(&config, block_time, ONE_FEEDER)
        );
    }

    #[test]
    fn less_feeders_with_valid_observations() {
        let validity_period = Duration::from_secs(60);
        let block_time = Timestamp::from_seconds(150);

        let feeder1 = Addr::unchecked("feeder1");
        let feed1_time = block_time - validity_period;
        let feed1_price = price(19, 5100);

        let mut feed = feed();
        feed = feed
            .add_observation(
                feeder1,
                feed1_time,
                feed1_price,
                &(feed1_time - validity_period),
            )
            .unwrap();

        let feeder2 = Addr::unchecked("feeder2");
        let feed2_time = block_time - validity_period + Duration::from_nanos(1);
        let feed2_price = price(19, 5000);
        feed = feed
            .add_observation(
                feeder2,
                feed2_time,
                feed2_price,
                &(feed2_time - validity_period),
            )
            .unwrap();

        let config = Config::new(
            Percent100::from_percent(50),
            SAMPLE_PERIOD,
            SAMPLES_NUMBER,
            DISCOUNTING_FACTOR,
        );
        assert_eq!(
            Ok(price(19, 5050)),
            feed.calc_price(&config, feed2_time, TWO_FEEDERS)
        );
        assert_eq!(
            Ok(price(19, 5050)),
            feed.calc_price(&config, block_time - Duration::from_nanos(1), TWO_FEEDERS)
        );
        assert_eq!(
            Ok(price(19, 5000)),
            feed.calc_price(&config, block_time, TWO_FEEDERS)
        );

        assert_eq!(
            Err(PriceFeedsError::NoPrice()),
            feed.calc_price(&config, block_time + Duration::from_nanos(1), TWO_FEEDERS)
        );
    }

    #[test]
    fn ema_price() {
        let block_time = Timestamp::from_seconds(100);
        let config = Config::new(
            Percent100::HUNDRED,
            SAMPLE_PERIOD,
            SAMPLES_NUMBER,
            DISCOUNTING_FACTOR,
        );

        let s1 = block_time - Duration::from_secs(12);
        let s21 = block_time - Duration::from_secs(7);
        let s22 = block_time - Duration::from_secs(6);
        let s3 = block_time - Duration::from_secs(2);

        let feeder1 = Addr::unchecked("feeder1");
        let feeder2 = Addr::unchecked("feeder2");

        let mut feed = feed();
        feed = feed
            .add_observation(
                feeder1.clone(),
                s1,
                price(19, 5160),
                &config.feed_valid_since(s1),
            )
            .unwrap();
        feed = feed
            .add_observation(
                feeder1.clone(),
                s21,
                price(19, 5500),
                &config.feed_valid_since(s21),
            )
            .unwrap();
        feed = feed
            .add_observation(
                feeder1.clone(),
                s22,
                price(19, 5000 + 10),
                &config.feed_valid_since(s22),
            )
            .unwrap();
        feed = feed
            .add_observation(
                feeder2,
                s22,
                price(19, 5000 - 10),
                &config.feed_valid_since(s22),
            )
            .unwrap();
        feed = feed
            .add_observation(feeder1, s3, price(19, 5000), &config.feed_valid_since(s3))
            .unwrap();

        assert_eq!(
            Ok(price(19, 5010)),
            feed.calc_price(&config, block_time, ONE_FEEDER)
        );
    }

    fn price(c: Amount, q: Amount) -> Price<TestC, TestQuoteC> {
        price::total_of(Coin::new(c)).is(Coin::new(q))
    }
}
