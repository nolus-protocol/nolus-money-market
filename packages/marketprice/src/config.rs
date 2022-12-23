use finance::{duration::Duration, fraction::Fraction, percent::Percent};
use sdk::schemars::{self, JsonSchema};
use serde::{Deserialize, Serialize};

use crate::error::{self, PriceFeedsError};

#[derive(Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
#[cfg_attr(
    any(test, feature = "testing"),
    derive(Debug, Clone),
    serde(into = "unchecked::Config")
)]
#[serde(try_from = "unchecked::Config")]
pub struct Config {
    min_feeders: Percent,
    sample_period: Duration,
    feed_validity: Duration,
    discount_factor: Percent,
}

impl Config {
    #[cfg(any(test, feature = "testing"))]
    pub fn new(
        feed_validity: Duration,
        min_feeders: Percent,
        sample_period: Duration,
        discount_factor: Percent,
    ) -> Self {
        let res = Config {
            min_feeders,
            sample_period,
            feed_validity,
            discount_factor,
        };
        debug_assert_eq!(Ok(()), res.invariant_held());
        res
    }

    pub fn min_feeders(&self, total: usize) -> usize {
        self.min_feeders.of(total)
    }

    pub fn sample_period(&self) -> Duration {
        self.sample_period
    }

    pub fn feed_validity(&self) -> Duration {
        // TODO make sure the feed_validity >= last block time
        self.feed_validity
    }

    pub fn discount_factor(&self) -> Percent {
        self.discount_factor
    }

    fn invariant_held(&self) -> Result<(), PriceFeedsError> {
        error::config_error_if(
            self.min_feeders == Percent::ZERO || self.min_feeders > Percent::HUNDRED,
            "The minumum feeders should be greater than 0 and less or equal to 100%",
        )?;

        error::config_error_if(
            self.feed_validity == Duration::default(),
            "The price feeds validity should be longer than zero",
        )?;

        error::config_error_if(
            self.sample_period == Duration::default(),
            "The sample period should be longer than zero",
        )?;

        error::config_error_if(
            self.sample_period > self.feed_validity,
            "The sample period should not be longer than the feeds validity",
        )?;

        error::config_error_if(
            self.discount_factor == Percent::ZERO || self.discount_factor > Percent::HUNDRED,
            "The discounting factor should be greater than 0 and less or equal to 100%",
        )?;

        Ok(())
    }
}

mod unchecked {
    use crate::error::PriceFeedsError;
    use finance::{duration::Duration, percent::Percent};
    use serde::{Deserialize, Serialize};

    use super::Config as ValidatedConfig;

    #[derive(Serialize, Deserialize)]
    pub(super) struct Config {
        min_feeders: Percent,
        sample_period_secs: u32,
        feed_validity_secs: u32,
        discount_factor: Percent,
    }

    impl From<ValidatedConfig> for Config {
        fn from(o: ValidatedConfig) -> Self {
            Self {
                min_feeders: o.min_feeders,
                sample_period_secs: expect_u32_secs(
                    o.sample_period,
                    "Programming error! The feed validity has been increased!",
                ),
                feed_validity_secs: expect_u32_secs(
                    o.feed_validity,
                    "Programming error! The feed validity has been increased!",
                ),
                discount_factor: o.discount_factor,
            }
        }
    }

    impl TryFrom<Config> for ValidatedConfig {
        type Error = PriceFeedsError;

        fn try_from(dto: Config) -> Result<Self, Self::Error> {
            let res = Self {
                min_feeders: dto.min_feeders,
                sample_period: Duration::from_secs(dto.sample_period_secs),
                feed_validity: Duration::from_secs(dto.feed_validity_secs),
                discount_factor: dto.discount_factor,
            };
            res.invariant_held()?;
            Ok(res)
        }
    }

    fn expect_u32_secs(d: Duration, descr: &str) -> u32 {
        d.secs().try_into().expect(descr)
    }
}

#[cfg(test)]
mod test {
    use finance::{duration::Duration, percent::Percent};
    use sdk::cosmwasm_std::{from_slice, to_vec, StdError};

    use crate::config::Config;

    fn min_feders_impl(min_feeders: u16, total: usize, exp: usize) {
        let c = Config::new(
            Duration::HOUR,
            Percent::from_percent(min_feeders),
            Duration::HOUR,
            Percent::from_percent(75),
        );
        assert_eq!(exp, c.min_feeders(total));
    }
    #[test]
    fn feeders_needed_rounds_properly() {
        min_feders_impl(100, 3, 3);
        min_feders_impl(100, 30, 30);
        min_feders_impl(50, 34, 17);
        min_feders_impl(50, 33, 16);

        min_feders_impl(25, 48, 12);
        min_feders_impl(25, 49, 12);
        min_feders_impl(25, 50, 12);
        min_feders_impl(25, 51, 12);
        min_feders_impl(25, 52, 13);

        min_feders_impl(1, 132, 1);
        min_feders_impl(1, 199, 1);
        min_feders_impl(1, 200, 2);
    }

    #[test]
    fn deserialize_invalid() {
        deserialize_fail(0, 10, 42, 22);
        deserialize_fail(1001, 10, 42, 22);
        deserialize_fail(10, 0, 42, 22);
        deserialize_fail(10, 43, 42, 22);
        deserialize_fail(10, 0, 0, 22);
        deserialize_fail(10, 43, 42, 0);
        deserialize_fail(10, 43, 42, 1001);
        deserialize_fail(1001, 10, 42, 22);
        deserialize_fail(10, 10, 42, 42122);
    }

    #[test]
    fn deserialize_valid() {
        deserialize_pass(1, 1, 1, 1);
        deserialize_pass(1000, 1, 1, 1);
        deserialize_pass(1, 1, 1, 1000);
        deserialize_pass(650, 5, 60, 750);
    }

    #[test]
    fn serde() {
        serde_impl(10, 1, 2, 800);
        serde_impl(1, 10, 10, 1);
        serde_impl(1, 10, 40, 123);
        serde_impl(1000, 2, 3, 1000);
        serde_impl(351, 13522, 13522, 750);
    }

    fn serde_impl(min_feeders: u32, sample_period: u32, feed_validity: u32, discount_factor: u32) {
        let c = Config::new(
            Duration::from_secs(feed_validity),
            Percent::from_permille(min_feeders),
            Duration::from_secs(sample_period),
            Percent::from_permille(discount_factor),
        );
        assert_eq!(from_slice(&to_vec(&c).unwrap()), Ok(c));
    }

    fn deserialize_pass(
        min_feeders: u32,
        sample_period: u32,
        feed_validity: u32,
        discount_factor: u32,
    ) {
        assert_eq!(
            Config::new(
                Duration::from_secs(feed_validity),
                Percent::from_permille(min_feeders),
                Duration::from_secs(sample_period),
                Percent::from_permille(discount_factor),
            ),
            deserialize(min_feeders, sample_period, feed_validity, discount_factor).unwrap()
        );
    }

    fn deserialize_fail(
        min_feeders: u32,
        sample_period: u32,
        feed_validity: u32,
        discount_factor: u32,
    ) {
        assert!(matches!(
            deserialize(min_feeders, sample_period, feed_validity, discount_factor).unwrap_err(),
            StdError::ParseErr { .. }
        ));
    }

    fn deserialize(
        min_feeders: u32,
        sample_period: u32,
        feed_validity: u32,
        discount_factor: u32,
    ) -> Result<Config, StdError> {
        from_slice(
            format!("{{\"min_feeders\": {min_feeders}, \"sample_period_secs\": {sample_period},\"feed_validity_secs\": {feed_validity}, \"discount_factor\": {discount_factor}}}")
                .as_bytes(),
        )
    }
}
