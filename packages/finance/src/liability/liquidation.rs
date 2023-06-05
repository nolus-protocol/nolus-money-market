use serde::{Deserialize, Serialize};

use crate::{
    coin::{Amount, Coin},
    currency::Currency,
    percent::Percent,
};

use super::{Liability, Zone};

const MIN_LIQUIDATION_AMOUNT: Amount = 10_000; // $0.01 TODO #40

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

pub fn check_liability<Asset>(
    spec: &Liability,
    asset: Coin<Asset>,
    total_due: Coin<Asset>,
    overdue: Coin<Asset>,
    min_asset_threshold: Coin<Asset>,
) -> Status<Asset>
where
    Asset: Currency,
{
    debug_assert!(total_due <= asset);
    debug_assert!(overdue <= total_due);
    may_ask_liquidation_liability(spec, asset, total_due, min_asset_threshold)
        .max(may_ask_liquidation_overdue(
            asset,
            overdue,
            min_asset_threshold,
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
    min_asset_threshold: Coin<Asset>,
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
        min_asset_threshold,
    )
}

fn may_ask_liquidation_overdue<Asset>(
    asset: Coin<Asset>,
    overdue: Coin<Asset>,
    min_asset_threshold: Coin<Asset>,
) -> Option<Status<Asset>>
where
    Asset: Currency,
{
    if overdue < Coin::new(MIN_LIQUIDATION_AMOUNT) {
        None
    } else {
        may_ask_liquidation(asset, Cause::Overdue(), overdue, min_asset_threshold)
    }
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
    use crate::{
        coin::{Amount, Coin},
        duration::Duration,
        percent::Percent,
        test::currency::Nls,
    };

    use super::{check_liability, Cause, Liability, Status, Zone};

    const MIN_DUE_AMOUNT: Coin<Nls> = Coin::new(10000);

    #[test]
    fn no_debt() {
        let warn_ltv = Percent::from_permille(11);
        let spec = liability_with_first(warn_ltv);
        assert_eq!(
            check_liability::<Nls>(&spec, 100.into(), 0.into(), 0.into(), 0.into()),
            Status::NoDebt,
        );
    }

    #[test]
    fn warnings_none() {
        let warn_ltv = Percent::from_percent(51);
        let spec = liability_with_first(warn_ltv);
        assert_eq!(
            check_liability::<Nls>(&spec, 100.into(), 1.into(), 0.into(), 0.into()),
            Status::No(Zone::no_warnings(spec.first_liq_warn())),
        );
        assert_eq!(
            check_liability::<Nls>(&spec, 100.into(), 49.into(), 0.into(), 0.into()),
            Status::No(Zone::no_warnings(spec.first_liq_warn())),
        );
        assert_eq!(
            check_liability::<Nls>(&spec, 100.into(), 50.into(), 0.into(), 0.into()),
            Status::No(Zone::no_warnings(spec.first_liq_warn())),
        );
        assert_eq!(
            check_liability::<Nls>(
                &spec,
                100000.into(),
                50500.into(),
                MIN_DUE_AMOUNT - 1.into(),
                0.into()
            ),
            Status::No(Zone::no_warnings(spec.first_liq_warn())),
        );
        assert_eq!(
            check_liability::<Nls>(&spec, 100000.into(), 50500.into(), MIN_DUE_AMOUNT, 0.into()),
            Status::partial(MIN_DUE_AMOUNT, Cause::Overdue()),
        );
        assert_eq!(
            check_liability::<Nls>(&spec, 1000.into(), 509.into(), 0.into(), 0.into()),
            Status::No(Zone::no_warnings(spec.first_liq_warn())),
        );
        assert_eq!(
            check_liability::<Nls>(&spec, 1000.into(), 510.into(), 0.into(), 0.into()),
            Status::No(Zone::first(spec.first_liq_warn(), spec.second_liq_warn())),
        );
        assert_eq!(
            check_liability::<Nls>(
                &spec,
                100000.into(),
                51000.into(),
                MIN_DUE_AMOUNT - 1.into(),
                0.into()
            ),
            Status::No(Zone::first(spec.first_liq_warn(), spec.second_liq_warn())),
        );
        assert_eq!(
            check_liability::<Nls>(&spec, 100000.into(), 51000.into(), MIN_DUE_AMOUNT, 0.into()),
            Status::partial(MIN_DUE_AMOUNT, Cause::Overdue()),
        );
    }

    #[test]
    fn warnings_first() {
        let spec = liability_with_first(Percent::from_permille(712));

        assert_eq!(
            check_liability::<Nls>(&spec, 1000.into(), 711.into(), 0.into(), 0.into()),
            Status::No(Zone::no_warnings(spec.first_liq_warn())),
        );
        assert_eq!(
            check_liability::<Nls>(&spec, 1000.into(), 712.into(), 0.into(), 0.into()),
            Status::No(Zone::first(spec.first_liq_warn(), spec.second_liq_warn())),
        );
        assert_eq!(
            check_liability::<Nls>(
                &spec,
                100000.into(),
                71200.into(),
                MIN_DUE_AMOUNT - 1.into(),
                0.into()
            ),
            Status::No(Zone::first(spec.first_liq_warn(), spec.second_liq_warn())),
        );
        assert_eq!(
            check_liability::<Nls>(&spec, 100000.into(), 71200.into(), MIN_DUE_AMOUNT, 0.into()),
            Status::partial(MIN_DUE_AMOUNT, Cause::Overdue())
        );
        assert_eq!(
            check_liability::<Nls>(&spec, 1000.into(), 715.into(), 0.into(), 0.into()),
            Status::No(Zone::first(spec.first_liq_warn(), spec.second_liq_warn())),
        );
        assert_eq!(
            check_liability::<Nls>(&spec, 1000.into(), 721.into(), 0.into(), 0.into()),
            Status::No(Zone::first(spec.first_liq_warn(), spec.second_liq_warn())),
        );
        assert_eq!(
            check_liability::<Nls>(&spec, 1000.into(), 722.into(), 0.into(), 0.into()),
            Status::No(Zone::second(spec.second_liq_warn(), spec.third_liq_warn())),
        );
    }

    #[test]
    fn warnings_second() {
        let spec = liability_with_second(Percent::from_permille(123));

        assert_eq!(
            check_liability::<Nls>(&spec, 1000.into(), 122.into(), 0.into(), 0.into()),
            Status::No(Zone::first(spec.first_liq_warn(), spec.second_liq_warn())),
        );
        assert_eq!(
            check_liability::<Nls>(&spec, 1000.into(), 123.into(), 0.into(), 0.into()),
            Status::No(Zone::second(spec.second_liq_warn(), spec.third_liq_warn())),
        );
        assert_eq!(
            check_liability::<Nls>(&spec, 1000.into(), 124.into(), 0.into(), 0.into()),
            Status::No(Zone::second(spec.second_liq_warn(), spec.third_liq_warn())),
        );
        assert_eq!(
            check_liability::<Nls>(
                &spec,
                100000.into(),
                12800.into(),
                MIN_DUE_AMOUNT - 1.into(),
                0.into()
            ),
            Status::No(Zone::second(spec.second_liq_warn(), spec.third_liq_warn())),
        );
        assert_eq!(
            check_liability::<Nls>(&spec, 100000.into(), 12800.into(), MIN_DUE_AMOUNT, 0.into()),
            Status::partial(MIN_DUE_AMOUNT, Cause::Overdue())
        );
        assert_eq!(
            check_liability::<Nls>(&spec, 1000.into(), 132.into(), 0.into(), 0.into()),
            Status::No(Zone::second(spec.second_liq_warn(), spec.third_liq_warn())),
        );
        assert_eq!(
            check_liability::<Nls>(&spec, 1000.into(), 133.into(), 0.into(), 0.into()),
            Status::No(Zone::third(spec.third_liq_warn(), spec.max())),
        );
    }

    #[test]
    fn warnings_third() {
        let warn_third_ltv = Percent::from_permille(381);
        let max_ltv = warn_third_ltv + STEP;
        let spec = liability_with_third(warn_third_ltv);

        assert_eq!(
            check_liability::<Nls>(&spec, 1000.into(), 380.into(), 0.into(), 0.into()),
            Status::No(Zone::second(spec.second_liq_warn(), spec.third_liq_warn())),
        );
        assert_eq!(
            check_liability::<Nls>(
                &spec,
                100000.into(),
                38000.into(),
                MIN_DUE_AMOUNT - 1.into(),
                0.into()
            ),
            Status::No(Zone::second(spec.second_liq_warn(), spec.third_liq_warn())),
        );
        assert_eq!(
            check_liability::<Nls>(&spec, 100000.into(), 38100.into(), MIN_DUE_AMOUNT, 0.into()),
            Status::partial(MIN_DUE_AMOUNT, Cause::Overdue())
        );
        assert_eq!(
            check_liability::<Nls>(&spec, 1000.into(), 381.into(), 0.into(), 0.into()),
            Status::No(Zone::third(spec.third_liq_warn(), spec.max())),
        );
        assert_eq!(
            check_liability::<Nls>(&spec, 1000.into(), 382.into(), 0.into(), 0.into()),
            Status::No(Zone::third(spec.third_liq_warn(), spec.max())),
        );
        assert_eq!(
            check_liability::<Nls>(&spec, 1000.into(), 390.into(), 0.into(), 0.into()),
            Status::No(Zone::third(spec.third_liq_warn(), spec.max())),
        );
        assert_eq!(
            check_liability::<Nls>(&spec, 1000.into(), 391.into(), 0.into(), 0.into()),
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
        const BACK_TO_HEALTHY: Amount = 87979;
        let spec = liability_with_max(max_ltv);

        assert_eq!(
            check_liability::<Nls>(
                &spec,
                100000.into(),
                88099.into(),
                MIN_DUE_AMOUNT - 1.into(),
                0.into()
            ),
            Status::No(Zone::third(spec.third_liq_warn(), spec.max())),
        );
        assert_eq!(
            check_liability::<Nls>(&spec, 100000.into(), 88099.into(), MIN_DUE_AMOUNT, 0.into()),
            Status::partial(MIN_DUE_AMOUNT, Cause::Overdue()),
        );
        assert_eq!(
            check_liability::<Nls>(&spec, 100000.into(), 88100.into(), MIN_DUE_AMOUNT, 0.into()),
            Status::partial(
                BACK_TO_HEALTHY.into(),
                Cause::Liability {
                    ltv: max_ltv,
                    healthy_ltv: STEP
                }
            ),
        );
        assert_eq!(
            check_liability::<Nls>(
                &spec,
                100000.into(),
                88100.into(),
                (BACK_TO_HEALTHY - 1).into(),
                0.into()
            ),
            Status::partial(
                BACK_TO_HEALTHY.into(),
                Cause::Liability {
                    ltv: max_ltv,
                    healthy_ltv: STEP
                }
            ),
        );
        assert_eq!(
            check_liability::<Nls>(
                &spec,
                100000.into(),
                88100.into(),
                BACK_TO_HEALTHY.into(),
                0.into()
            ),
            Status::partial(
                BACK_TO_HEALTHY.into(),
                Cause::Liability {
                    ltv: max_ltv,
                    healthy_ltv: STEP
                }
            ),
        );
        assert_eq!(
            check_liability::<Nls>(
                &spec,
                100000.into(),
                88100.into(),
                (BACK_TO_HEALTHY + 1).into(),
                0.into()
            ),
            Status::partial((BACK_TO_HEALTHY + 1).into(), Cause::Overdue()),
        );
        assert_eq!(
            check_liability::<Nls>(&spec, 1000.into(), 999.into(), 997.into(), 0.into()),
            Status::partial(
                998.into(),
                Cause::Liability {
                    ltv: max_ltv,
                    healthy_ltv: STEP
                }
            ),
        );
        assert_eq!(
            check_liability::<Nls>(&spec, 1000.into(), 1000.into(), 1.into(), 0.into()),
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
            check_liability::<Nls>(&spec, 100000.into(), 76800.into(), 76565.into(), 0.into()),
            Status::partial(
                76565.into(),
                Cause::Liability {
                    ltv: max_ltv,
                    healthy_ltv: STEP
                }
            ),
        );
        assert_eq!(
            check_liability::<Nls>(&spec, 100000.into(), 76800.into(), 76566.into(), 0.into()),
            Status::partial(76566.into(), Cause::Overdue()),
        );
        assert_eq!(
            check_liability::<Nls>(&spec, 1000.into(), 1000.into(), 1.into(), 0.into()),
            Status::full(Cause::Liability {
                ltv: max_ltv,
                healthy_ltv: STEP
            }),
        );
    }

    #[test]
    fn liquidate_full_with_threshold() {
        let max_ltv = Percent::from_permille(573);
        let spec = liability_with_max(max_ltv);

        const BACK_TO_HEALTHY: Amount = 89898;
        assert_eq!(
            check_liability::<Nls>(
                &spec,
                100000.into(),
                90000.into(),
                BACK_TO_HEALTHY.into(),
                (100000 - BACK_TO_HEALTHY - 1).into()
            ),
            Status::partial(
                BACK_TO_HEALTHY.into(),
                Cause::Liability {
                    ltv: max_ltv,
                    healthy_ltv: STEP
                }
            ),
        );
        assert_eq!(
            check_liability::<Nls>(
                &spec,
                100000.into(),
                90000.into(),
                (BACK_TO_HEALTHY + 1).into(),
                (100000 - BACK_TO_HEALTHY - 2).into()
            ),
            Status::partial((BACK_TO_HEALTHY + 1).into(), Cause::Overdue()),
        );
        assert_eq!(
            check_liability::<Nls>(
                &spec,
                100000.into(),
                57299.into(),
                MIN_DUE_AMOUNT - 1.into(),
                100000.into()
            ),
            Status::No(Zone::third(spec.third_liq_warn(), spec.max())),
        );
        assert_eq!(
            check_liability::<Nls>(
                &spec,
                100000.into(),
                90000.into(),
                BACK_TO_HEALTHY.into(),
                (100000 - BACK_TO_HEALTHY).into()
            ),
            Status::full(Cause::Liability {
                ltv: max_ltv,
                healthy_ltv: STEP
            }),
        );
        assert_eq!(
            check_liability::<Nls>(
                &spec,
                100000.into(),
                90000.into(),
                (BACK_TO_HEALTHY + 1).into(),
                (100000 - BACK_TO_HEALTHY - 1).into()
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
