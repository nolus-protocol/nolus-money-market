use std::ops::Range;

use serde::{Deserialize, Serialize};

use finance::{coin::Coin, duration::Duration, liability::Zone, percent::Percent};

#[derive(Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[cfg_attr(test, derive(Debug))]
pub enum Cause {
    Overdue(),
    Liability { ltv: Percent, healthy_ltv: Percent },
}

#[derive(Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
#[cfg_attr(test, derive(Debug))]
pub enum Liquidation<Asset> {
    Partial { amount: Coin<Asset>, cause: Cause },
    Full(Cause),
}

#[derive(Clone, Copy, Eq, PartialEq)]
#[cfg_attr(test, derive(Debug))]
pub enum Debt<Asset> {
    No,
    /// Represent an open position with no immediate close required
    Ok {
        /// The position's debt results to an LTV% within the `liability` zone
        liability: Zone,
        /// The position would be steady, i.e. no automatic close, if its LTV% falls within the provided range.
        /// The `steadiness` is always a sub-range of the `liability` zone's range.
        steadiness: Range<Percent>,
        /// When is recommended to check again the position debt
        recheck_in: Duration,
    },
    Bad(Liquidation<Asset>),
}

impl<Asset> Debt<Asset> {
    #[cfg(test)]
    pub(crate) fn curable(sell_to_cover_debt: Coin<Asset>, cause: Cause) -> Self {
        debug_assert!(!sell_to_cover_debt.is_zero());
        Self::Bad(Liquidation::Partial {
            amount: sell_to_cover_debt,
            cause,
        })
    }

    #[cfg(test)]
    pub(crate) fn unmanageable(cause: Cause) -> Self {
        Self::Bad(Liquidation::Full(cause))
    }
}

#[cfg(test)]
mod test_status {
    use currencies::Lpn;
    use finance::percent::Percent;

    use crate::position::Liquidation;

    use super::Cause;

    #[test]
    fn ord_liq() {
        assert!(
            Liquidation::<Lpn>::Full(Cause::Overdue())
                < Liquidation::Full(Cause::Liability {
                    ltv: Percent::from_percent(20),
                    healthy_ltv: Percent::from_percent(40)
                })
        );
        assert!(
            Liquidation::<Lpn>::Full(Cause::Liability {
                ltv: Percent::from_percent(19),
                healthy_ltv: Percent::from_percent(40)
            }) < Liquidation::Full(Cause::Liability {
                ltv: Percent::from_percent(20),
                healthy_ltv: Percent::from_percent(40)
            })
        )
    }
}
