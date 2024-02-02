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
pub enum Debt<Asset>
where
    Asset: Currency,
{
    No,
    Ok { zone: Zone, recheck_in: Duration },
    Bad(Liquidation<Asset>),
}

impl<Asset> Debt<Asset>
where
    Asset: Currency,
{
    #[cfg(test)]
    pub(crate) fn ok(zone: Zone, recheck_in: Duration) -> Self {
        Self::Ok { zone, recheck_in }
    }

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
    use currencies::test::StableC1;
    use finance::{duration::Duration, percent::Percent};

    use super::{Cause, Debt, Zone};

    #[test]
    fn ord() {
        let recheck_in = Duration::HOUR;

        assert!(
            Debt::<StableC1>::No
                < Debt::<StableC1>::ok(Zone::no_warnings(Percent::from_permille(1)), recheck_in)
        );
        assert!(
            Debt::<StableC1>::ok(Zone::no_warnings(Percent::from_permille(1)), recheck_in)
                < Debt::ok(
                    Zone::first(Percent::from_permille(1), Percent::from_permille(2)),
                    recheck_in
                )
        );
        assert!(
            Debt::<StableC1>::ok(
                Zone::first(Percent::from_permille(1), Percent::from_permille(2)),
                recheck_in
            ) < Debt::ok(
                Zone::second(Percent::from_permille(1), Percent::from_permille(2)),
                recheck_in
            )
        );
        assert!(
            Debt::<StableC1>::ok(
                Zone::first(Percent::from_permille(1), Percent::from_permille(2)),
                recheck_in
            ) < Debt::ok(
                Zone::first(Percent::from_permille(1), Percent::from_permille(3)),
                recheck_in
            )
        );
        assert!(
            Debt::ok(
                Zone::first(Percent::from_permille(2), Percent::from_permille(3)),
                recheck_in
            ) < Debt::<StableC1>::ok(
                Zone::second(Percent::from_permille(1), Percent::from_permille(2)),
                recheck_in
            )
        );
        assert!(
            Debt::ok(
                Zone::third(Percent::from_permille(991), Percent::from_permille(1000)),
                recheck_in
            ) < Debt::<StableC1>::partial(1.into(), Cause::Overdue())
        );
        assert!(
            Debt::<StableC1>::partial(1.into(), Cause::Overdue())
                < Debt::<StableC1>::partial(
                    1.into(),
                    Cause::Liability {
                        ltv: Percent::from_permille(1),
                        healthy_ltv: Percent::from_permille(1)
                    }
                )
        );
        assert!(
            Debt::<StableC1>::partial(1.into(), Cause::Overdue())
                < Debt::<StableC1>::partial(2.into(), Cause::Overdue())
        );
        assert!(
            Debt::<StableC1>::partial(
                1.into(),
                Cause::Liability {
                    ltv: Percent::from_permille(1),
                    healthy_ltv: Percent::from_permille(1)
                }
            ) < Debt::<StableC1>::partial(
                2.into(),
                Cause::Liability {
                    ltv: Percent::from_permille(1),
                    healthy_ltv: Percent::from_permille(1)
                }
            )
        );
        assert!(
            Debt::<StableC1>::partial(
                1.into(),
                Cause::Liability {
                    ltv: Percent::from_permille(1),
                    healthy_ltv: Percent::from_permille(1)
                }
            ) < Debt::<StableC1>::partial(
                1.into(),
                Cause::Liability {
                    ltv: Percent::from_permille(1),
                    healthy_ltv: Percent::from_permille(2)
                }
            )
        );
        assert!(
            Debt::<StableC1>::partial(
                1.into(),
                Cause::Liability {
                    ltv: Percent::from_permille(1),
                    healthy_ltv: Percent::from_permille(1)
                }
            ) < Debt::<StableC1>::full(Cause::Liability {
                ltv: Percent::from_permille(1),
                healthy_ltv: Percent::from_permille(2)
            })
        );
        assert!(
            Debt::<StableC1>::full(Cause::Liability {
                ltv: Percent::from_permille(1),
                healthy_ltv: Percent::from_permille(1)
            }) < Debt::<StableC1>::full(Cause::Liability {
                ltv: Percent::from_permille(1),
                healthy_ltv: Percent::from_permille(2)
            })
        );
        assert!(
            Debt::<StableC1>::full(Cause::Overdue())
                < Debt::<StableC1>::full(Cause::Liability {
                    ltv: Percent::from_permille(1),
                    healthy_ltv: Percent::from_permille(1)
                })
        );
    }
}
