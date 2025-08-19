use crate::{
    percent::Percent100,
    range::{Ascending, RightOpenRange},
};

use super::Level;

/// Liability zone is an interval a lease LTV belongs to.
///
/// Zones cover only the leases that are not pending a liquidation.
///
/// A zone is defined as a right-open interval of LTVs between two Levels.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(any(test, feature = "testing"), derive(Debug))]
pub struct Zone {
    low: Option<Level>,
    high: Level,
}

impl Zone {
    pub const fn no_warnings(up_to: Percent100) -> Self {
        Self {
            low: None,
            high: Level::First(up_to),
        }
    }

    pub fn first(low: Percent100, high: Percent100) -> Self {
        debug_assert!(low < high);
        Self {
            low: Some(Level::First(low)),
            high: Level::Second(high),
        }
    }

    pub fn second(low: Percent100, high: Percent100) -> Self {
        debug_assert!(low < high);
        Self {
            low: Some(Level::Second(low)),
            high: Level::Third(high),
        }
    }

    pub fn third(low: Percent100, high: Percent100) -> Self {
        debug_assert!(low < high);
        Self {
            low: Some(Level::Third(low)),
            high: Level::Max(high),
        }
    }

    pub fn range(&self) -> RightOpenRange<Percent100, Ascending> {
        let range_to = RightOpenRange::up_to(self.high.ltv());
        self.low.map_or(range_to, |low| range_to.cut_to(low.ltv()))
    }

    pub const fn low(&self) -> Option<Level> {
        self.low
    }
}

#[cfg(test)]
mod test {
    use crate::{liability::Zone, percent::Percent100, range::RightOpenRange};

    #[test]
    fn ord() {
        assert!(
            Zone::no_warnings(Percent100::HUNDRED)
                < Zone::first(Percent100::from_percent(0), Percent100::from_percent(10))
        );
        assert!(
            Zone::first(Percent100::from_percent(0), Percent100::from_percent(10))
                < Zone::first(Percent100::from_percent(0), Percent100::from_percent(11))
        );
        assert!(
            Zone::first(Percent100::from_percent(0), Percent100::from_percent(10))
                < Zone::first(Percent100::from_percent(5), Percent100::from_percent(6))
        );
        assert!(
            Zone::first(Percent100::from_percent(23), Percent100::from_percent(24))
                < Zone::second(Percent100::from_percent(0), Percent100::from_percent(10))
        );
    }

    #[test]
    fn range() {
        let above = Percent100::from_percent(23);
        let below = Percent100::from_percent(34);
        assert_eq!(
            RightOpenRange::up_to(below),
            Zone::no_warnings(below).range()
        );

        assert_eq!(
            RightOpenRange::up_to(below).cut_to(above),
            Zone::second(above, below).range()
        );
    }
}
