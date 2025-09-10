use std::{cmp, collections::HashMap};

use finance::{duration::Duration, fraction::Fraction, price::Price, ratio::Ratio};
use sdk::cosmwasm_std::{Addr, Timestamp};

use super::observation::Observation;

/// Builds an infinite iterator of samples
///
/// It loops over #Observation instances, groups them by time in periods,
/// takes the last by feeder, and computes an average for each period.
/// If there are no observations for a period, the sample from the last
/// period is yielded again.
pub fn from_observations<'a, Observations, C, QuoteC>(
    observations: Observations,
    start_from: Timestamp,
    sample_span: Duration,
) -> impl Iterator<Item = Sample<C, QuoteC>>
where
    Observations: Iterator<Item = &'a Observation<C, QuoteC>>,
    C: 'static,
    QuoteC: 'static,
{
    SampleBuilder::from(observations, start_from, sample_span)
}

#[cfg_attr(test, derive(PartialEq, Eq, Debug))]
pub struct Sample<C, QuoteC>
where
    C: 'static,
    QuoteC: 'static,
{
    /// Sample with no price means there has not been enough valid data to compute it.
    /// For example, none feed has been received within the validity window.
    price: Option<Price<C, QuoteC>>,
}

impl<C, QuoteC> Default for Sample<C, QuoteC>
where
    C: 'static,
    QuoteC: 'static,
{
    fn default() -> Self {
        Self { price: None }
    }
}

impl<C, QuoteC> Clone for Sample<C, QuoteC>
where
    C: 'static,
    QuoteC: 'static,
{
    fn clone(&self) -> Self {
        *self
    }
}

impl<C, QuoteC> Copy for Sample<C, QuoteC>
where
    C: 'static,
    QuoteC: 'static,
{
}

impl<C, QuoteC> Sample<C, QuoteC> {
    pub fn into_maybe_price(self) -> Option<Price<C, QuoteC>> {
        self.price
    }
}

struct SampleBuilder<'a, IterO, C, QuoteC>
where
    IterO: Iterator<Item = &'a Observation<C, QuoteC>>,
    C: 'static,
    QuoteC: 'static,
{
    observations: IterO,
    sample_start: Timestamp,
    sample_span: Duration,
    consumed: Option<IterO::Item>,
    sample_prices: HashMap<&'a Addr, Price<C, QuoteC>>,
    last_sample: <Self as Iterator>::Item,
}

impl<'a, IterO, C, QuoteC> SampleBuilder<'a, IterO, C, QuoteC>
where
    IterO: Iterator<Item = &'a Observation<C, QuoteC>>,
{
    fn from(observations: IterO, start_from: Timestamp, sample_span: Duration) -> Self {
        Self {
            observations,
            sample_start: start_from,
            sample_span,
            consumed: None,
            sample_prices: HashMap::default(),
            last_sample: <Self as Iterator>::Item::default(),
        }
    }

    fn sample_end(&self) -> Timestamp {
        self.sample_start + self.sample_span
    }

    fn end_of_period(&mut self) {
        let prices_number = Self::prices_number(self.sample_prices.len());

        if prices_number > 0 {
            let mut values = self.sample_prices.values();
            let first = values
                .next()
                .expect("should have been checked that there is at least one member");

            let sum = values
                .take(prices_number - 1)
                .fold(*first, |acc, current| acc + *current);

            let part = Ratio::new(
                1,
                u128::try_from(prices_number)
                    .expect("prices_number is already restricted to fit in u128::MAX"),
            );
            let avg = Fraction::of(&part, sum);
            self.last_sample = Sample { price: Some(avg) };
        }

        self.sample_prices.clear();
        self.sample_start = self.sample_end();
    }

    fn prices_number(len: usize) -> usize {
        cmp::min(len, u128::MAX.try_into().unwrap_or(usize::MAX))
    }
}

impl<'a, IterO, C, QuoteC> Iterator for SampleBuilder<'a, IterO, C, QuoteC>
where
    C: 'static,
    QuoteC: 'static,
    IterO: Iterator<Item = &'a Observation<C, QuoteC>>,
{
    type Item = Sample<C, QuoteC>;

    fn next(&mut self) -> Option<Self::Item> {
        let sample_end = self.sample_end();

        let pipeline = self.consumed.into_iter().chain(&mut self.observations);
        self.consumed = None;

        for o in pipeline {
            debug_assert!(o.valid_since(&self.sample_start));
            if o.valid_since(&sample_end) {
                self.consumed = Some(o);
                break;
            } else {
                self.sample_prices.insert(o.feeder(), o.price());
            }
        }
        self.end_of_period();
        Some(self.last_sample)
    }
}

#[cfg(test)]
mod test {
    use currency::test::{SuperGroupTestC4, SuperGroupTestC5};
    use finance::{coin::Amount, duration::Duration, price};
    use sdk::cosmwasm_std::{Addr, Timestamp};

    use crate::feed::{
        observation::Observation,
        sample::{self, Sample},
    };

    type TheCurrency = SuperGroupTestC4;
    type TheQuote = SuperGroupTestC5;

    #[test]
    fn one_observation() {
        let start_from = Timestamp::from_seconds(150);
        let t1 = Timestamp::from_seconds(200);
        let p1 = price(1, 12000);
        let obs = [Observation::new(feeder1(), t1, p1)];

        let mut samples =
            sample::from_observations(obs.iter(), start_from, Duration::from_secs(25));
        assert_eq!(Some(Sample::default()), samples.next());
        assert_eq!(Some(Sample { price: Some(p1) }), samples.next());
        assert_eq!(Some(Sample { price: Some(p1) }), samples.next());
        assert_eq!(Some(Sample { price: Some(p1) }), samples.next());
    }

    #[test]
    fn a_few_observations_per_feeder() {
        let start_from = Timestamp::from_seconds(150);
        let t11 = Timestamp::from_seconds(160);
        let t21 = Timestamp::from_seconds(180);
        let t22 = Timestamp::from_seconds(200);
        let p1 = price(1, 12000);
        let p2 = price(1, 13000);
        let p3 = price(1, 14000);
        let obs = vec![
            Observation::new(feeder1(), t11, p1), // first period
            Observation::new(feeder1(), t21, p2), // second period
            Observation::new(feeder2(), t21, p3),
            Observation::new(feeder1(), t22, p3),
        ];

        let mut samples =
            sample::from_observations(obs.iter(), start_from, Duration::from_secs(25));
        assert_eq!(Some(Sample { price: Some(p1) }), samples.next());
        assert_eq!(Some(Sample { price: Some(p3) }), samples.next());
        assert_eq!(Some(Sample { price: Some(p3) }), samples.next());
        assert_eq!(Some(Sample { price: Some(p3) }), samples.next());
    }

    #[test]
    fn real_observations() {
        let start_from = Timestamp::from_seconds(150);
        let t11 = Timestamp::from_seconds(160);
        let t21 = Timestamp::from_seconds(180);
        let t22 = Timestamp::from_seconds(200);
        let t31 = Timestamp::from_seconds(201);
        let t32 = Timestamp::from_seconds(225);

        let p1 = price(1, 12000);
        let p2 = price(1, 13000);
        let p3 = price(1, 14000);
        let p13 = p2;
        let p23 = price(1, 13500);
        let obs = vec![
            Observation::new(feeder1(), t11, p1), // first period
            Observation::new(feeder1(), t11, p3),
            Observation::new(feeder2(), t11, p1),
            Observation::new(feeder1(), t21, p2), // second period
            Observation::new(feeder2(), t21, p3),
            Observation::new(feeder1(), t22, p2),
            Observation::new(feeder2(), t31, p2), // third period
            Observation::new(feeder2(), t32, p1),
            Observation::new(feeder1(), t32, p1),
        ];

        let mut samples =
            sample::from_observations(obs.iter(), start_from, Duration::from_secs(25));
        assert_eq!(Some(Sample { price: Some(p13) }), samples.next());
        assert_eq!(Some(Sample { price: Some(p23) }), samples.next());
        assert_eq!(Some(Sample { price: Some(p1) }), samples.next());
        assert_eq!(Some(Sample { price: Some(p1) }), samples.next());
    }

    fn price(of: Amount, is: Amount) -> price::Price<TheCurrency, TheQuote> {
        price::total_of(of.into()).is(is.into())
    }

    fn feeder1() -> Addr {
        Addr::unchecked("feeder1")
    }

    fn feeder2() -> Addr {
        Addr::unchecked("feeder2")
    }
}
