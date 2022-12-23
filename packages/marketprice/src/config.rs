use finance::{duration::Duration, fraction::Fraction, percent::Percent};
use sdk::schemars::{self, JsonSchema};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct Config {
    feed_validity: Duration,
    min_feeders: Percent,
}

impl Config {
    pub fn new(feed_validity: Duration, min_feeders: Percent) -> Self {
        Config {
            feed_validity,
            min_feeders,
        }
    }

    pub fn min_feeders(&self, total: usize) -> usize {
        self.min_feeders.of(total)
    }

    pub fn feed_validity(&self) -> Duration {
        self.feed_validity
    }
}

#[cfg(test)]
mod test {
    use finance::{duration::Duration, percent::Percent};

    use crate::config::Config;

    fn min_feders_impl(min_feeders: u16, total: usize, exp: usize) {
        let c = Config::new(Duration::HOUR, Percent::from_percent(min_feeders));
        assert_eq!(exp, c.min_feeders(total));
    }
    #[test]
    fn feeders_needed_rounds_properly() {
        min_feders_impl(255, 3, 7);
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
}
