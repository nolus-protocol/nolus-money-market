use std::cmp::Ordering;

use serde::{Deserialize, Serialize};

use finance::{coin::Coin, duration::Duration, liability::Zone, percent::Percent};

#[derive(Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[cfg_attr(test, derive(Debug))]
pub enum Cause {
    Overdue(),
    Liability { ltv: Percent, healthy_ltv: Percent },
}

// #[derive(Clone, Copy, Eq, Ord, PartialEq, PartialOrd)]
#[derive(Clone, Copy)]
#[cfg_attr(test, derive(Debug))]
pub enum Liquidation<Asset> {
    Partial { amount: Coin<Asset>, cause: Cause },
    Full(Cause),
}

#[derive(Clone, Copy, Eq, PartialEq)]
#[cfg_attr(test, derive(Debug))]
pub enum Debt<Asset> {
    No,
    Ok { zone: Zone, recheck_in: Duration },
    Bad(Liquidation<Asset>),
}

impl<Asset> Debt<Asset> {
    #[cfg(test)]
    pub(crate) fn partial(amount: Coin<Asset>, cause: Cause) -> Self {
        debug_assert!(!amount.is_zero());
        Self::Bad(Liquidation::Partial { amount, cause })
    }

    #[cfg(test)]
    pub(crate) fn full(cause: Cause) -> Self {
        Self::Bad(Liquidation::Full(cause))
    }
}

#[cfg(test)]
mod test_status {
    use currencies::test::LpnC;
    use finance::percent::Percent;

    use crate::position::Liquidation;

    use super::Cause;

    #[test]
    fn ord_liq() {
        assert!(
            Liquidation::<LpnC>::Full(Cause::Overdue())
                < Liquidation::Full(Cause::Liability {
                    ltv: Percent::from_percent(20),
                    healthy_ltv: Percent::from_percent(40)
                })
        );
        assert!(
            Liquidation::<LpnC>::Full(Cause::Liability {
                ltv: Percent::from_percent(19),
                healthy_ltv: Percent::from_percent(40)
            }) < Liquidation::Full(Cause::Liability {
                ltv: Percent::from_percent(20),
                healthy_ltv: Percent::from_percent(40)
            })
        )
    }
}
