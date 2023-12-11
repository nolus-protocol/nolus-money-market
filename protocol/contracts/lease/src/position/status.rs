use serde::{Deserialize, Serialize};

use currency::Currency;
use finance::{coin::Coin, duration::Duration, liability::Zone, percent::Percent};

#[derive(Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[cfg_attr(test, derive(Debug))]
pub enum Cause {
    Overdue(),
    Liability { ltv: Percent, healthy_ltv: Percent },
}

#[derive(Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
#[cfg_attr(test, derive(Debug))]
pub enum Liquidation<Asset>
where
    Asset: Currency,
{
    Partial { amount: Coin<Asset>, cause: Cause },
    Full(Cause),
}

#[derive(Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
#[cfg_attr(test, derive(Debug))]
pub enum Status<Asset>
where
    Asset: Currency,
{
    NoDebt,
    No { zone: Zone, recalc_in: Duration },
    Liquidation(Liquidation<Asset>),
}

impl<Asset> Status<Asset>
where
    Asset: Currency,
{
    #[cfg(test)]
    pub(crate) fn partial(amount: Coin<Asset>, cause: Cause) -> Self {
        debug_assert!(!amount.is_zero());
        Self::Liquidation(Liquidation::Partial { amount, cause })
    }

    #[cfg(test)]
    pub(crate) fn full(cause: Cause) -> Self {
        Self::Liquidation(Liquidation::Full(cause))
    }
}

#[cfg(test)]
mod test_status {
    use currencies::test::StableC1;
    use finance::{duration::Duration, percent::Percent};

    use super::{Cause, Liquidation, Status, Zone};

    #[test]
    fn ord() {
        let recalc_in = Duration::HOUR;

        assert!(
            Status::<StableC1>::No {
                zone: Zone::no_warnings(Percent::from_permille(1)),
                recalc_in
            } < Status::No {
                zone: Zone::first(Percent::from_permille(1), Percent::from_permille(2)),
                recalc_in
            }
        );
        assert!(
            Status::<StableC1>::No {
                zone: Zone::first(Percent::from_permille(1), Percent::from_permille(2)),
                recalc_in
            } < Status::No {
                zone: Zone::second(Percent::from_permille(1), Percent::from_permille(2)),
                recalc_in
            }
        );
        assert!(
            Status::<StableC1>::No {
                zone: Zone::first(Percent::from_permille(1), Percent::from_permille(2)),
                recalc_in
            } < Status::No {
                zone: Zone::first(Percent::from_permille(1), Percent::from_permille(3)),
                recalc_in
            }
        );
        assert!(
            Status::No {
                zone: Zone::first(Percent::from_permille(2), Percent::from_permille(3)),
                recalc_in
            } < Status::<StableC1>::No {
                zone: Zone::second(Percent::from_permille(1), Percent::from_permille(2)),
                recalc_in
            }
        );
        assert!(
            Status::No {
                zone: Zone::third(Percent::from_permille(991), Percent::from_permille(1000)),
                recalc_in
            } < Status::<StableC1>::Liquidation(Liquidation::Partial {
                amount: 1.into(),
                cause: Cause::Overdue()
            })
        );
        assert!(
            Status::<StableC1>::Liquidation(Liquidation::Partial {
                amount: 1.into(),
                cause: Cause::Overdue()
            }) < Status::<StableC1>::Liquidation(Liquidation::Partial {
                amount: 1.into(),
                cause: Cause::Liability {
                    ltv: Percent::from_permille(1),
                    healthy_ltv: Percent::from_permille(1)
                }
            })
        );
        assert!(
            Status::<StableC1>::Liquidation(Liquidation::Partial {
                amount: 1.into(),
                cause: Cause::Overdue()
            }) < Status::<StableC1>::Liquidation(Liquidation::Partial {
                amount: 2.into(),
                cause: Cause::Overdue()
            })
        );
        assert!(
            Status::<StableC1>::Liquidation(Liquidation::Partial {
                amount: 1.into(),
                cause: Cause::Liability {
                    ltv: Percent::from_permille(1),
                    healthy_ltv: Percent::from_permille(1)
                }
            }) < Status::<StableC1>::Liquidation(Liquidation::Partial {
                amount: 2.into(),
                cause: Cause::Liability {
                    ltv: Percent::from_permille(1),
                    healthy_ltv: Percent::from_permille(1)
                }
            })
        );
        assert!(
            Status::<StableC1>::partial(
                1.into(),
                Cause::Liability {
                    ltv: Percent::from_permille(1),
                    healthy_ltv: Percent::from_permille(1)
                }
            ) < Status::<StableC1>::partial(
                1.into(),
                Cause::Liability {
                    ltv: Percent::from_permille(1),
                    healthy_ltv: Percent::from_permille(2)
                }
            )
        );
        assert!(
            Status::<StableC1>::partial(
                1.into(),
                Cause::Liability {
                    ltv: Percent::from_permille(1),
                    healthy_ltv: Percent::from_permille(1)
                }
            ) < Status::<StableC1>::full(Cause::Liability {
                ltv: Percent::from_permille(1),
                healthy_ltv: Percent::from_permille(2)
            })
        );
        assert!(
            Status::<StableC1>::full(Cause::Liability {
                ltv: Percent::from_permille(1),
                healthy_ltv: Percent::from_permille(1)
            }) < Status::<StableC1>::full(Cause::Liability {
                ltv: Percent::from_permille(1),
                healthy_ltv: Percent::from_permille(2)
            })
        );
        assert!(
            Status::<StableC1>::full(Cause::Overdue())
                < Status::<StableC1>::full(Cause::Liability {
                    ltv: Percent::from_permille(1),
                    healthy_ltv: Percent::from_permille(1)
                })
        );
    }
}
