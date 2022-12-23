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
    feed_validity: Duration,
    min_feeders: Percent,
}

impl Config {
    #[cfg(any(test, feature = "testing"))]
    pub fn new(feed_validity: Duration, min_feeders: Percent) -> Self {
        let res = Config {
            feed_validity,
            min_feeders,
        };
        debug_assert_eq!(Ok(()), res.invariant_held());
        res
    }

    pub fn min_feeders(&self, total: usize) -> usize {
        self.min_feeders.of(total)
    }

    pub fn feed_validity(&self) -> Duration {
        self.feed_validity
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
        // TODO make sure the price_feed_period >= last block time

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
        feed_validity_secs: u32,
        min_feeders: Percent,
    }

    impl From<ValidatedConfig> for Config {
        fn from(o: ValidatedConfig) -> Self {
            Self {
                feed_validity_secs: o
                    .feed_validity
                    .secs()
                    .try_into()
                    .expect("Programming error! The feed validity is increased!"),
                min_feeders: o.min_feeders,
            }
        }
    }

    impl TryFrom<Config> for ValidatedConfig {
        type Error = PriceFeedsError;

        fn try_from(dto: Config) -> Result<Self, Self::Error> {
            let res = Self {
                feed_validity: Duration::from_secs(dto.feed_validity_secs),
                min_feeders: dto.min_feeders,
            };
            res.invariant_held()?;
            Ok(res)
        }
    }
}

#[cfg(test)]
mod test {
    use finance::{duration::Duration, percent::Percent};
    use sdk::cosmwasm_std::{from_slice, to_vec, StdError};

    use crate::config::Config;

    fn min_feders_impl(min_feeders: u16, total: usize, exp: usize) {
        let c = Config::new(Duration::HOUR, Percent::from_percent(min_feeders));
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
        deserialize_fail(0, 10);
        deserialize_fail(10, 0);
        deserialize_fail(0, 1001);
    }

    #[test]
    fn deserialize_valid() {
        deserialize_pass(1, 10);
        deserialize_pass(10, 1);
        deserialize_pass(1, 1000);
    }

    #[test]
    fn serde() {
        serde_impl(1, 10);
        serde_impl(10, 1);
        serde_impl(1, 1000);
        serde_impl(13522, 351);
    }

    fn serde_impl(feed_validity: u32, min_feeders: u32) {
        let c = Config::new(
            Duration::from_secs(feed_validity),
            Percent::from_permille(min_feeders),
        );
        assert_eq!(from_slice(&to_vec(&c).unwrap()), Ok(c));
    }

    fn deserialize_pass(feed_validity: u32, min_feeders: u32) {
        assert_eq!(
            Config::new(
                Duration::from_secs(feed_validity),
                Percent::from_permille(min_feeders)
            ),
            deserialize(feed_validity, min_feeders).unwrap()
        );
    }

    fn deserialize_fail(feed_validity: u32, min_feeders: u32) {
        assert!(matches!(
            deserialize(feed_validity, min_feeders).unwrap_err(),
            StdError::ParseErr { .. }
        ));
    }

    fn deserialize(feed_validity: u32, min_feeders: u32) -> Result<Config, StdError> {
        from_slice(
            format!("{{\"feed_validity_secs\": {feed_validity}, \"min_feeders\": {min_feeders}}}")
                .as_bytes(),
        )
    }
}
