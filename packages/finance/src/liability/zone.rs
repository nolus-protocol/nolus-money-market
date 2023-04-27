use crate::percent::Percent;

use super::Level;

/// Liability zone representing a key property of a lease that is not pending a liquidation.
///
/// A zone is defined as an interval of LTVs between two Levels.
#[derive(Clone, Copy, PartialEq, Eq)]
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
