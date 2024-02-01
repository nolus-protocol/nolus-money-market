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
    // Lpn: Currency,
{
    No,
    Ok {
        zone: Zone,
        recheck_in: Duration,
        // TODO
        //  collect_overdue_in: Duration,
        //  price_low: Price<Asset, Lpn>,
        //  price_high: Price<Asset, Lpn>,
    },
    Bad(Liquidation<Asset>),
}

impl<Asset> Debt<Asset>
where
    Asset: Currency,
{
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

    use super::{Cause, Debt, Liquidation, Zone};

    #[test]
    fn ord() {
        let recheck_in = Duration::HOUR;

        assert!(
            Debt::<StableC1>::Ok {
                zone: Zone::no_warnings(Percent::from_permille(1)),
                recheck_in
            } < Debt::Ok {
                zone: Zone::first(Percent::from_permille(1), Percent::from_permille(2)),
                recheck_in
            }
        );
        assert!(
            Debt::<StableC1>::Ok {
                zone: Zone::first(Percent::from_permille(1), Percent::from_permille(2)),
                recheck_in
            } < Debt::Ok {
                zone: Zone::second(Percent::from_permille(1), Percent::from_permille(2)),
                recheck_in
            }
        );
        assert!(
            Debt::<StableC1>::Ok {
                zone: Zone::first(Percent::from_permille(1), Percent::from_permille(2)),
                recheck_in
            } < Debt::Ok {
                zone: Zone::first(Percent::from_permille(1), Percent::from_permille(3)),
                recheck_in
            }
        );
        assert!(
            Debt::Ok {
                zone: Zone::first(Percent::from_permille(2), Percent::from_permille(3)),
                recheck_in
            } < Debt::<StableC1>::Ok {
                zone: Zone::second(Percent::from_permille(1), Percent::from_permille(2)),
                recheck_in
            }
        );
        assert!(
            Debt::Ok {
                zone: Zone::third(Percent::from_permille(991), Percent::from_permille(1000)),
                recheck_in
            } < Debt::<StableC1>::Bad(Liquidation::Partial {
                amount: 1.into(),
                cause: Cause::Overdue()
            })
        );
        assert!(
            Debt::<StableC1>::Bad(Liquidation::Partial {
                amount: 1.into(),
                cause: Cause::Overdue()
            }) < Debt::<StableC1>::Bad(Liquidation::Partial {
                amount: 1.into(),
                cause: Cause::Liability {
                    ltv: Percent::from_permille(1),
                    healthy_ltv: Percent::from_permille(1)
                }
            })
        );
        assert!(
            Debt::<StableC1>::Bad(Liquidation::Partial {
                amount: 1.into(),
                cause: Cause::Overdue()
            }) < Debt::<StableC1>::Bad(Liquidation::Partial {
                amount: 2.into(),
                cause: Cause::Overdue()
            })
        );
        assert!(
            Debt::<StableC1>::Bad(Liquidation::Partial {
                amount: 1.into(),
                cause: Cause::Liability {
                    ltv: Percent::from_permille(1),
                    healthy_ltv: Percent::from_permille(1)
                }
            }) < Debt::<StableC1>::Bad(Liquidation::Partial {
                amount: 2.into(),
                cause: Cause::Liability {
                    ltv: Percent::from_permille(1),
                    healthy_ltv: Percent::from_permille(1)
                }
            })
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
