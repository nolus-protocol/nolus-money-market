use serde::{Deserialize, Serialize};

use crate::{coin::Coin, currency::Currency, percent::Percent};

use super::{Liability, Zone};

#[derive(Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[cfg_attr(test, derive(Debug))]
pub enum Cause {
    Overdue(),
    Liability { ltv: Percent, healthy_ltv: Percent },
}

#[derive(Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
#[cfg_attr(test, derive(Debug))]
pub enum Status<Asset>
where
    Asset: Currency,
{
    NoDebt,
    No(Zone),
    Liquidation(Liquidation<Asset>),
}

impl<Asset> Status<Asset>
where
    Asset: Currency,
{
    fn partial(amount: Coin<Asset>, cause: Cause) -> Self {
        Self::Liquidation(Liquidation::Partial { amount, cause })
    }

    fn full(cause: Cause) -> Self {
        Self::Liquidation(Liquidation::Full(cause))
    }

    #[cfg(debug_assertion)]
    fn amount<Lpn, Lpp, Profit, TimeAlarms, Oracle>(
        &self,
        lease: &Lease<Lpn, Asset, Lpp, Profit, TimeAlarms, Oracle>,
    ) -> Coin<Asset> {
        match self {
            Self::No(_) => Default::default(),
            Self::Liquidation(liq) => liq.amount(lease),
        }
    }
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

impl<Asset> Liquidation<Asset>
where
    Asset: Currency,
{
    #[cfg(debug_assertion)]
    pub(crate) fn amount<Lpn, Lpp, Profit, TimeAlarms, Oracle>(
        &self,
        lease: &Lease<Lpn, Asset, Lpp, Profit, TimeAlarms, Oracle>,
    ) -> Coin<Asset> {
        match self {
            Self::Partial { amount, cause: _ } => *amount,
            Self::Full(_) => lease.amount,
        }
    }
}

pub fn inspect_liability<Asset>(
    spec: &Liability,
    asset: Coin<Asset>,
    total_due: Coin<Asset>,
    overdue: Coin<Asset>,
    liquidation_threshold: Coin<Asset>,
) -> Status<Asset>
where
    Asset: Currency,
{
    may_ask_liquidation_liability(spec, asset, total_due, liquidation_threshold)
        .max(may_ask_liquidation_overdue(
            asset,
            overdue,
            liquidation_threshold,
        ))
        .unwrap_or_else(|| no_liquidation(spec, asset, total_due))
}

fn no_liquidation<Asset>(
    spec: &Liability,
    asset: Coin<Asset>,
    total_due: Coin<Asset>,
) -> Status<Asset>
where
    Asset: Currency,
{
    if total_due.is_zero() {
        Status::NoDebt
    } else {
        let ltv = Percent::from_ratio(total_due, asset);
        debug_assert!(ltv < spec.max());

        Status::No(spec.zone_of(ltv))
    }
}

fn may_ask_liquidation_liability<Asset>(
    spec: &Liability,
    asset: Coin<Asset>,
    total_due: Coin<Asset>,
    liquidation_threshold: Coin<Asset>,
) -> Option<Status<Asset>>
where
    Asset: Currency,
{
    may_ask_liquidation(
        asset,
        Cause::Liability {
            ltv: spec.max(),
            healthy_ltv: spec.healthy_percent(),
        },
        spec.amount_to_liquidate(asset, total_due),
        liquidation_threshold,
    )
}

fn may_ask_liquidation_overdue<Asset>(
    asset: Coin<Asset>,
    overdue: Coin<Asset>,
    liquidation_threshold: Coin<Asset>,
) -> Option<Status<Asset>>
where
    Asset: Currency,
{
    may_ask_liquidation(asset, Cause::Overdue(), overdue, liquidation_threshold)
}

fn may_ask_liquidation<Asset>(
    asset: Coin<Asset>,
    cause: Cause,
    liquidation: Coin<Asset>,
    liquidation_threshold: Coin<Asset>,
) -> Option<Status<Asset>>
where
    Asset: Currency,
{
    if liquidation.is_zero() {
        None
    } else if asset.saturating_sub(liquidation) <= liquidation_threshold {
        Some(Status::full(cause))
    } else {
        Some(Status::partial(liquidation, cause))
    }
}

#[cfg(test)]
mod tests {
    use crate::{coin::Amount, duration::Duration, percent::Percent, test::currency::Nls};

    use super::{inspect_liability, Cause, Liability, Status, Zone};

    #[test]
    fn no_debt() {
        let warn_ltv = Percent::from_permille(11);
        let spec = liability_with_first(warn_ltv);
        assert_eq!(
            inspect_liability::<Nls>(&spec, 100.into(), 0.into(), 0.into(), 0.into()),
            Status::NoDebt,
        );
    }

    #[test]
    fn warnings_none() {
        let warn_ltv = Percent::from_percent(51);
        let spec = liability_with_first(warn_ltv);
        assert_eq!(
            inspect_liability::<Nls>(&spec, 100.into(), 1.into(), 0.into(), 0.into()),
            Status::No(Zone::no_warnings(spec.first_liq_warn())),
        );
        assert_eq!(
            inspect_liability::<Nls>(&spec, 100.into(), 49.into(), 0.into(), 0.into()),
            Status::No(Zone::no_warnings(spec.first_liq_warn())),
        );
        assert_eq!(
            inspect_liability::<Nls>(&spec, 100.into(), 50.into(), 0.into(), 0.into()),
            Status::No(Zone::no_warnings(spec.first_liq_warn())),
        );
        assert_eq!(
            inspect_liability::<Nls>(&spec, 1000.into(), 505.into(), 1.into(), 0.into()),
            Status::partial(1.into(), Cause::Overdue()),
        );
        assert_eq!(
            inspect_liability::<Nls>(&spec, 1000.into(), 509.into(), 0.into(), 0.into()),
            Status::No(Zone::no_warnings(spec.first_liq_warn())),
        );
        assert_eq!(
            inspect_liability::<Nls>(&spec, 1000.into(), 510.into(), 0.into(), 0.into()),
            Status::No(Zone::first(spec.first_liq_warn(), spec.second_liq_warn())),
        );
        assert_eq!(
            inspect_liability::<Nls>(&spec, 1000.into(), 510.into(), 1.into(), 0.into()),
            Status::partial(1.into(), Cause::Overdue()),
        );
    }

    #[test]
    fn warnings_first() {
        let spec = liability_with_first(Percent::from_permille(712));

        assert_eq!(
            inspect_liability::<Nls>(&spec, 1000.into(), 711.into(), 0.into(), 0.into()),
            Status::No(Zone::no_warnings(spec.first_liq_warn())),
        );
        assert_eq!(
            inspect_liability::<Nls>(&spec, 1000.into(), 712.into(), 0.into(), 0.into()),
            Status::No(Zone::first(spec.first_liq_warn(), spec.second_liq_warn())),
        );
        assert_eq!(
            inspect_liability::<Nls>(&spec, 1000.into(), 712.into(), 1.into(), 0.into()),
            Status::partial(1.into(), Cause::Overdue())
        );
        assert_eq!(
            inspect_liability::<Nls>(&spec, 1000.into(), 715.into(), 0.into(), 0.into()),
            Status::No(Zone::first(spec.first_liq_warn(), spec.second_liq_warn())),
        );
        assert_eq!(
            inspect_liability::<Nls>(&spec, 1000.into(), 721.into(), 0.into(), 0.into()),
            Status::No(Zone::first(spec.first_liq_warn(), spec.second_liq_warn())),
        );
        assert_eq!(
            inspect_liability::<Nls>(&spec, 1000.into(), 722.into(), 0.into(), 0.into()),
            Status::No(Zone::second(spec.second_liq_warn(), spec.third_liq_warn())),
        );
    }

    #[test]
    fn warnings_second() {
        let spec = liability_with_second(Percent::from_permille(123));

        assert_eq!(
            inspect_liability::<Nls>(&spec, 1000.into(), 122.into(), 0.into(), 0.into()),
            Status::No(Zone::first(spec.first_liq_warn(), spec.second_liq_warn())),
        );
        assert_eq!(
            inspect_liability::<Nls>(&spec, 1000.into(), 123.into(), 0.into(), 0.into()),
            Status::No(Zone::second(spec.second_liq_warn(), spec.third_liq_warn())),
        );
        assert_eq!(
            inspect_liability::<Nls>(&spec, 1000.into(), 124.into(), 0.into(), 0.into()),
            Status::No(Zone::second(spec.second_liq_warn(), spec.third_liq_warn())),
        );
        assert_eq!(
            inspect_liability::<Nls>(&spec, 1000.into(), 128.into(), 1.into(), 0.into()),
            Status::partial(1.into(), Cause::Overdue())
        );
        assert_eq!(
            inspect_liability::<Nls>(&spec, 1000.into(), 132.into(), 0.into(), 0.into()),
            Status::No(Zone::second(spec.second_liq_warn(), spec.third_liq_warn())),
        );
        assert_eq!(
            inspect_liability::<Nls>(&spec, 1000.into(), 133.into(), 0.into(), 0.into()),
            Status::No(Zone::third(spec.third_liq_warn(), spec.max())),
        );
    }

    #[test]
    fn warnings_third() {
        let warn_third_ltv = Percent::from_permille(381);
        let max_ltv = warn_third_ltv + STEP;
        let spec = liability_with_third(warn_third_ltv);

        assert_eq!(
            inspect_liability::<Nls>(&spec, 1000.into(), 380.into(), 0.into(), 0.into()),
            Status::No(Zone::second(spec.second_liq_warn(), spec.third_liq_warn())),
        );
        assert_eq!(
            inspect_liability::<Nls>(&spec, 1000.into(), 381.into(), 1.into(), 0.into()),
            Status::partial(1.into(), Cause::Overdue())
        );
        assert_eq!(
            inspect_liability::<Nls>(&spec, 1000.into(), 381.into(), 0.into(), 0.into()),
            Status::No(Zone::third(spec.third_liq_warn(), spec.max())),
        );
        assert_eq!(
            inspect_liability::<Nls>(&spec, 1000.into(), 382.into(), 0.into(), 0.into()),
            Status::No(Zone::third(spec.third_liq_warn(), spec.max())),
        );
        assert_eq!(
            inspect_liability::<Nls>(&spec, 1000.into(), 390.into(), 0.into(), 0.into()),
            Status::No(Zone::third(spec.third_liq_warn(), spec.max())),
        );
        assert_eq!(
            inspect_liability::<Nls>(&spec, 1000.into(), 391.into(), 0.into(), 0.into()),
            Status::partial(
                384.into(),
                Cause::Liability {
                    ltv: max_ltv,
                    healthy_ltv: STEP
                }
            ),
        );
    }

    #[test]
    fn liquidate_partial() {
        let max_ltv = Percent::from_permille(881);
        let spec = liability_with_max(max_ltv);

        assert_eq!(
            inspect_liability::<Nls>(&spec, 1000.into(), 880.into(), 0.into(), 0.into()),
            Status::No(Zone::third(spec.third_liq_warn(), spec.max())),
        );
        assert_eq!(
            inspect_liability::<Nls>(&spec, 1000.into(), 880.into(), 1.into(), 0.into()),
            Status::partial(1.into(), Cause::Overdue()),
        );
        assert_eq!(
            inspect_liability::<Nls>(&spec, 1000.into(), 881.into(), 0.into(), 0.into()),
            Status::partial(
                879.into(),
                Cause::Liability {
                    ltv: max_ltv,
                    healthy_ltv: STEP
                }
            ),
        );
        assert_eq!(
            inspect_liability::<Nls>(&spec, 1000.into(), 881.into(), 878.into(), 0.into()),
            Status::partial(
                879.into(),
                Cause::Liability {
                    ltv: max_ltv,
                    healthy_ltv: STEP
                }
            ),
        );
        assert_eq!(
            inspect_liability::<Nls>(&spec, 1000.into(), 881.into(), 879.into(), 0.into()),
            Status::partial(
                879.into(),
                Cause::Liability {
                    ltv: max_ltv,
                    healthy_ltv: STEP
                }
            ),
        );
        assert_eq!(
            inspect_liability::<Nls>(&spec, 1000.into(), 881.into(), 880.into(), 0.into()),
            Status::partial(880.into(), Cause::Overdue()),
        );
        assert_eq!(
            inspect_liability::<Nls>(&spec, 1000.into(), 999.into(), 997.into(), 0.into()),
            Status::partial(
                998.into(),
                Cause::Liability {
                    ltv: max_ltv,
                    healthy_ltv: STEP
                }
            ),
        );
        assert_eq!(
            inspect_liability::<Nls>(&spec, 1000.into(), 1000.into(), 1.into(), 0.into()),
            Status::full(Cause::Liability {
                ltv: max_ltv,
                healthy_ltv: STEP
            }),
        );
    }

    #[test]
    fn liquidate_full() {
        let max_ltv = Percent::from_permille(768);
        let spec = liability_with_max(max_ltv);

        assert_eq!(
            inspect_liability::<Nls>(&spec, 1000.into(), 768.into(), 765.into(), 0.into()),
            Status::partial(
                765.into(),
                Cause::Liability {
                    ltv: max_ltv,
                    healthy_ltv: STEP
                }
            ),
        );
        assert_eq!(
            inspect_liability::<Nls>(&spec, 1000.into(), 768.into(), 766.into(), 0.into()),
            Status::partial(766.into(), Cause::Overdue()),
        );
        assert_eq!(
            inspect_liability::<Nls>(&spec, 1000.into(), 1000.into(), 1.into(), 0.into()),
            Status::full(Cause::Liability {
                ltv: max_ltv,
                healthy_ltv: STEP
            }),
        );
        let back_to_healthy: Amount = spec.amount_to_liquidate(1000, 900);
        assert_eq!(
            inspect_liability::<Nls>(
                &spec,
                1000.into(),
                900.into(),
                back_to_healthy.into(),
                (1000 - back_to_healthy - 1).into()
            ),
            Status::partial(
                back_to_healthy.into(),
                Cause::Liability {
                    ltv: max_ltv,
                    healthy_ltv: STEP
                }
            ),
        );
        assert_eq!(
            inspect_liability::<Nls>(
                &spec,
                1000.into(),
                900.into(),
                (back_to_healthy + 1).into(),
                (1000 - back_to_healthy - 2).into()
            ),
            Status::partial((back_to_healthy + 1).into(), Cause::Overdue()),
        );
        assert_eq!(
            inspect_liability::<Nls>(
                &spec,
                1000.into(),
                900.into(),
                back_to_healthy.into(),
                (1000 - back_to_healthy).into()
            ),
            Status::full(Cause::Liability {
                ltv: max_ltv,
                healthy_ltv: STEP
            }),
        );
        assert_eq!(
            inspect_liability::<Nls>(
                &spec,
                1000.into(),
                900.into(),
                (back_to_healthy + 1).into(),
                (1000 - back_to_healthy - 1).into()
            ),
            Status::full(Cause::Overdue()),
        );
    }

    const STEP: Percent = Percent::from_permille(10);

    fn liability_with_first(warn: Percent) -> Liability {
        liability_with_max(warn + STEP + STEP + STEP)
    }

    fn liability_with_second(warn: Percent) -> Liability {
        liability_with_max(warn + STEP + STEP)
    }

    fn liability_with_third(warn: Percent) -> Liability {
        liability_with_max(warn + STEP)
    }

    // init = 1%, healthy = 1%, first = max - 3, second = max - 2, third = max - 1
    fn liability_with_max(max: Percent) -> Liability {
        let initial = STEP;
        assert!(initial < max - STEP - STEP - STEP);

        Liability::new(
            initial,
            Percent::ZERO,
            max - initial,
            STEP,
            STEP,
            STEP,
            Duration::from_hours(1),
        )
    }
}

#[cfg(test)]
mod test_status {
    use crate::{percent::Percent, test::currency::Usdc};

    use super::{Cause, Liquidation, Status, Zone};

    #[test]
    fn ord() {
        assert!(
            Status::<Usdc>::No(Zone::no_warnings(Percent::from_permille(1)))
                < Status::No(Zone::first(
                    Percent::from_permille(1),
                    Percent::from_permille(2)
                ))
        );
        assert!(
            Status::<Usdc>::No(Zone::first(
                Percent::from_permille(1),
                Percent::from_permille(2)
            )) < Status::No(Zone::second(
                Percent::from_permille(1),
                Percent::from_permille(2)
            ))
        );
        assert!(
            Status::<Usdc>::No(Zone::first(
                Percent::from_permille(1),
                Percent::from_permille(2)
            )) < Status::No(Zone::first(
                Percent::from_permille(1),
                Percent::from_permille(3)
            ))
        );
        assert!(
            Status::No(Zone::first(
                Percent::from_permille(2),
                Percent::from_permille(3)
            )) < Status::<Usdc>::No(Zone::second(
                Percent::from_permille(1),
                Percent::from_permille(2)
            ))
        );
        assert!(
            Status::No(Zone::third(
                Percent::from_permille(991),
                Percent::from_permille(1000)
            )) < Status::<Usdc>::Liquidation(Liquidation::Partial {
                amount: 1.into(),
                cause: Cause::Overdue()
            })
        );
        assert!(
            Status::<Usdc>::Liquidation(Liquidation::Partial {
                amount: 1.into(),
                cause: Cause::Overdue()
            }) < Status::<Usdc>::Liquidation(Liquidation::Partial {
                amount: 1.into(),
                cause: Cause::Liability {
                    ltv: Percent::from_permille(1),
                    healthy_ltv: Percent::from_permille(1)
                }
            })
        );
        assert!(
            Status::<Usdc>::Liquidation(Liquidation::Partial {
                amount: 1.into(),
                cause: Cause::Overdue()
            }) < Status::<Usdc>::Liquidation(Liquidation::Partial {
                amount: 2.into(),
                cause: Cause::Overdue()
            })
        );
        assert!(
            Status::<Usdc>::Liquidation(Liquidation::Partial {
                amount: 1.into(),
                cause: Cause::Liability {
                    ltv: Percent::from_permille(1),
                    healthy_ltv: Percent::from_permille(1)
                }
            }) < Status::<Usdc>::Liquidation(Liquidation::Partial {
                amount: 2.into(),
                cause: Cause::Liability {
                    ltv: Percent::from_permille(1),
                    healthy_ltv: Percent::from_permille(1)
                }
            })
        );
        assert!(
            Status::<Usdc>::partial(
                1.into(),
                Cause::Liability {
                    ltv: Percent::from_permille(1),
                    healthy_ltv: Percent::from_permille(1)
                }
            ) < Status::<Usdc>::partial(
                1.into(),
                Cause::Liability {
                    ltv: Percent::from_permille(1),
                    healthy_ltv: Percent::from_permille(2)
                }
            )
        );
        assert!(
            Status::<Usdc>::partial(
                1.into(),
                Cause::Liability {
                    ltv: Percent::from_permille(1),
                    healthy_ltv: Percent::from_permille(1)
                }
            ) < Status::<Usdc>::full(Cause::Liability {
                ltv: Percent::from_permille(1),
                healthy_ltv: Percent::from_permille(2)
            })
        );
        assert!(
            Status::<Usdc>::full(Cause::Liability {
                ltv: Percent::from_permille(1),
                healthy_ltv: Percent::from_permille(1)
            }) < Status::<Usdc>::full(Cause::Liability {
                ltv: Percent::from_permille(1),
                healthy_ltv: Percent::from_permille(2)
            })
        );
        assert!(
            Status::<Usdc>::full(Cause::Overdue())
                < Status::<Usdc>::full(Cause::Liability {
                    ltv: Percent::from_permille(1),
                    healthy_ltv: Percent::from_permille(1)
                })
        );
    }
}
