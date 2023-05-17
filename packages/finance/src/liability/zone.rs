use crate::percent::Percent;

use super::Level;

/// Liability zone is an interval a lease LTV belongs to.
///
/// Zones cover only the leases that are not pending a liquidation.
///
/// A zone is defined as an interval of LTVs between two Levels.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(any(test, feature = "testing"), derive(Debug))]
pub struct Zone {
    low: Option<Level>,
    high: Level,
}

impl Zone {
    pub fn no_warnings(up_to: Percent) -> Self {
        Self {
            low: None,
            high: Level::First(up_to),
        }
    }

    pub fn first(low: Percent, high: Percent) -> Self {
        debug_assert!(low < high);
        Self {
            low: Some(Level::First(low)),
            high: Level::Second(high),
        }
    }

    pub fn second(low: Percent, high: Percent) -> Self {
        debug_assert!(low < high);
        Self {
            low: Some(Level::Second(low)),
            high: Level::Third(high),
        }
    }

    pub fn third(low: Percent, high: Percent) -> Self {
        debug_assert!(low < high);
        Self {
            low: Some(Level::Third(low)),
            high: Level::Max(high),
        }
    }

    pub fn low(&self) -> Option<Level> {
        self.low
    }

    pub fn high(&self) -> Level {
        self.high
    }
}

#[cfg(test)]
mod test {
    use crate::{liability::Zone, percent::Percent};

    #[test]
    fn ord() {
        assert!(
            Zone::no_warnings(Percent::HUNDRED)
                < Zone::first(Percent::from_percent(0), Percent::from_percent(10))
        );
        assert!(
            Zone::first(Percent::from_percent(0), Percent::from_percent(10))
                < Zone::first(Percent::from_percent(0), Percent::from_percent(11))
        );
        assert!(
            Zone::first(Percent::from_percent(0), Percent::from_percent(10))
                < Zone::first(Percent::from_percent(5), Percent::from_percent(6))
        );
        assert!(
            Zone::first(Percent::from_percent(23), Percent::from_percent(24))
                < Zone::second(Percent::from_percent(0), Percent::from_percent(10))
        );
    }
}
