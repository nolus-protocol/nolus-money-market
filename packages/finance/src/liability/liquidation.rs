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

pub fn check_liability<Asset>(
    spec: &Liability,
    asset: Coin<Asset>,
    total_due: Coin<Asset>,
    overdue: Coin<Asset>,
    min_liquidation: Coin<Asset>,
    min_asset: Coin<Asset>,
) -> Status<Asset>
where
    Asset: Currency,
{
    debug_assert!(total_due <= asset);
    debug_assert!(overdue <= total_due);
    may_ask_liquidation_liability(spec, asset, total_due, min_asset)
        .max(may_ask_liquidation_overdue(
            asset,
            overdue,
            min_liquidation,
            min_asset,
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
    min_asset: Coin<Asset>,
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
            min_asset,
        )
    }    

fn may_ask_liquidation_overdue<Asset>(
    asset: Coin<Asset>,
    overdue: Coin<Asset>,
    min_liquidation: Coin<Asset>,
    min_asset: Coin<Asset>,
) -> Option<Status<Asset>>
where
    Asset: Currency,
{
    if overdue < min_liquidation {
        None
    } else {
        may_ask_liquidation(asset, Cause::Overdue(), overdue, min_asset)
    }
}

fn may_ask_liquidation<Asset>(
    asset: Coin<Asset>,
    cause: Cause,
    liquidation: Coin<Asset>,
    min_asset: Coin<Asset>,
) -> Option<Status<Asset>>
where
    Asset: Currency,
{
    if liquidation.is_zero() {
        None
    } else if asset.saturating_sub(liquidation) <= min_asset {
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

    const MIN_DUE_AMOUNT: Coin<Nls> = Coin::new(100);
    const LEASE_AMOUNT: Coin<Nls> = Coin::new(1000);

    #[test]
    fn no_debt() {
        let warn_ltv = Percent::from_permille(11);
        let spec = liability_with_first(warn_ltv);
        assert_eq!(
            check_liability::<Nls>(
                &spec,
                100.into(),
                0.into(),
                0.into(),
                MIN_DUE_AMOUNT,
                0.into()
            ),
            Status::NoDebt,
        );
    }

    #[test]
    fn warnings_none() {
        let warn_ltv = Percent::from_percent(51);
        let spec = liability_with_first(warn_ltv);
        assert_eq!(
            check_liability::<Nls>(
                &spec,
                100.into(),
                1.into(),
                0.into(),
                MIN_DUE_AMOUNT,
                0.into()
            ),
            Status::No(Zone::no_warnings(spec.first_liq_warn())),
        );
        assert_eq!(
            check_liability::<Nls>(
                &spec,
                100.into(),
                49.into(),
                0.into(),
                MIN_DUE_AMOUNT,
                0.into()
            ),
            Status::No(Zone::no_warnings(spec.first_liq_warn())),
        );
        assert_eq!(
            check_liability::<Nls>(
                &spec,
                100.into(),
                50.into(),
                0.into(),
                MIN_DUE_AMOUNT,
                0.into()
            ),
            Status::No(Zone::no_warnings(spec.first_liq_warn())),
        );
        assert_eq!(
            check_liability::<Nls>(
                &spec,
                LEASE_AMOUNT,
                505.into(),
                MIN_DUE_AMOUNT - 1.into(),
                MIN_DUE_AMOUNT,
                0.into()
            ),
            Status::No(Zone::no_warnings(spec.first_liq_warn())),
        );
        assert_eq!(
            check_liability::<Nls>(
                &spec,
                LEASE_AMOUNT,
                505.into(),
                MIN_DUE_AMOUNT,
                MIN_DUE_AMOUNT,
                0.into()
            ),
            Status::partial(MIN_DUE_AMOUNT, Cause::Overdue()),
        );
        assert_eq!(
            check_liability::<Nls>(
                &spec,
                LEASE_AMOUNT,
                509.into(),
                0.into(),
                MIN_DUE_AMOUNT,
                0.into()
            ),
            Status::No(Zone::no_warnings(spec.first_liq_warn())),
        );
        assert_eq!(
            check_liability::<Nls>(
                &spec,
                LEASE_AMOUNT,
                510.into(),
                0.into(),
                MIN_DUE_AMOUNT,
                0.into()
            ),
            Status::No(Zone::first(spec.first_liq_warn(), spec.second_liq_warn())),
        );
        assert_eq!(
            check_liability::<Nls>(
                &spec,
                LEASE_AMOUNT,
                510.into(),
                MIN_DUE_AMOUNT - 1.into(),
                MIN_DUE_AMOUNT,
                0.into()
            ),
            Status::No(Zone::first(spec.first_liq_warn(), spec.second_liq_warn())),
        );
        assert_eq!(
            check_liability::<Nls>(
                &spec,
                LEASE_AMOUNT,
                510.into(),
                MIN_DUE_AMOUNT,
                MIN_DUE_AMOUNT,
                0.into()
            ),
            Status::partial(MIN_DUE_AMOUNT, Cause::Overdue()),
        );
    }

    #[test]
    fn warnings_first() {
        let spec = liability_with_first(Percent::from_permille(712));

        assert_eq!(
            check_liability::<Nls>(
                &spec,
                LEASE_AMOUNT,
                711.into(),
                0.into(),
                MIN_DUE_AMOUNT,
                0.into()
            ),
            Status::No(Zone::no_warnings(spec.first_liq_warn())),
        );
        assert_eq!(
            check_liability::<Nls>(
                &spec,
                LEASE_AMOUNT,
                712.into(),
                0.into(),
                MIN_DUE_AMOUNT,
                0.into()
            ),
            Status::No(Zone::first(spec.first_liq_warn(), spec.second_liq_warn())),
        );
        assert_eq!(
            check_liability::<Nls>(
                &spec,
                LEASE_AMOUNT,
                712.into(),
                MIN_DUE_AMOUNT - 1.into(),
                MIN_DUE_AMOUNT,
                0.into()
            ),
            Status::No(Zone::first(spec.first_liq_warn(), spec.second_liq_warn())),
        );
        assert_eq!(
            check_liability::<Nls>(
                &spec,
                LEASE_AMOUNT,
                712.into(),
                MIN_DUE_AMOUNT,
                MIN_DUE_AMOUNT,
                0.into()
            ),
            Status::partial(MIN_DUE_AMOUNT, Cause::Overdue())
        );
        assert_eq!(
            check_liability::<Nls>(
                &spec,
                LEASE_AMOUNT,
                715.into(),
                0.into(),
                MIN_DUE_AMOUNT,
                0.into()
            ),
            Status::No(Zone::first(spec.first_liq_warn(), spec.second_liq_warn())),
        );
        assert_eq!(
            check_liability::<Nls>(
                &spec,
                LEASE_AMOUNT,
                721.into(),
                0.into(),
                MIN_DUE_AMOUNT,
                0.into()
            ),
            Status::No(Zone::first(spec.first_liq_warn(), spec.second_liq_warn())),
        );
        assert_eq!(
            check_liability::<Nls>(
                &spec,
                LEASE_AMOUNT,
                722.into(),
                0.into(),
                MIN_DUE_AMOUNT,
                0.into()
            ),
            Status::No(Zone::second(spec.second_liq_warn(), spec.third_liq_warn())),
        );
    }

    #[test]
    fn warnings_second() {
        let spec = liability_with_second(Percent::from_permille(123));

        assert_eq!(
            check_liability::<Nls>(
                &spec,
                LEASE_AMOUNT,
                122.into(),
                0.into(),
                MIN_DUE_AMOUNT,
                0.into()
            ),
            Status::No(Zone::first(spec.first_liq_warn(), spec.second_liq_warn())),
        );
        assert_eq!(
            check_liability::<Nls>(
                &spec,
                LEASE_AMOUNT,
                123.into(),
                0.into(),
                MIN_DUE_AMOUNT,
                0.into()
            ),
            Status::No(Zone::second(spec.second_liq_warn(), spec.third_liq_warn())),
        );
        assert_eq!(
            check_liability::<Nls>(
                &spec,
                LEASE_AMOUNT,
                124.into(),
                0.into(),
                MIN_DUE_AMOUNT,
                0.into()
            ),
            Status::No(Zone::second(spec.second_liq_warn(), spec.third_liq_warn())),
        );
        assert_eq!(
            check_liability::<Nls>(
                &spec,
                LEASE_AMOUNT,
                128.into(),
                MIN_DUE_AMOUNT - 1.into(),
                MIN_DUE_AMOUNT,
                0.into()
            ),
            Status::No(Zone::second(spec.second_liq_warn(), spec.third_liq_warn())),
        );
        assert_eq!(
            check_liability::<Nls>(
                &spec,
                LEASE_AMOUNT,
                128.into(),
                MIN_DUE_AMOUNT,
                MIN_DUE_AMOUNT,
                0.into()
            ),
            Status::partial(MIN_DUE_AMOUNT, Cause::Overdue())
        );
        assert_eq!(
            check_liability::<Nls>(
                &spec,
                LEASE_AMOUNT,
                132.into(),
                0.into(),
                MIN_DUE_AMOUNT,
                0.into()
            ),
            Status::No(Zone::second(spec.second_liq_warn(), spec.third_liq_warn())),
        );
        assert_eq!(
            check_liability::<Nls>(
                &spec,
                LEASE_AMOUNT,
                133.into(),
                0.into(),
                MIN_DUE_AMOUNT,
                0.into()
            ),
            Status::No(Zone::third(spec.third_liq_warn(), spec.max())),
        );
    }

    #[test]
    fn warnings_third() {
        let warn_third_ltv = Percent::from_permille(381);
        let max_ltv = warn_third_ltv + STEP;
        let spec = liability_with_third(warn_third_ltv);

        assert_eq!(
            check_liability::<Nls>(
                &spec,
                LEASE_AMOUNT,
                380.into(),
                0.into(),
                MIN_DUE_AMOUNT,
                0.into()
            ),
            Status::No(Zone::second(spec.second_liq_warn(), spec.third_liq_warn())),
        );
        assert_eq!(
            check_liability::<Nls>(
                &spec,
                LEASE_AMOUNT,
                380.into(),
                MIN_DUE_AMOUNT - 1.into(),
                MIN_DUE_AMOUNT,
                0.into()
            ),
            Status::No(Zone::second(spec.second_liq_warn(), spec.third_liq_warn())),
        );
        assert_eq!(
            check_liability::<Nls>(
                &spec,
                LEASE_AMOUNT,
                381.into(),
                MIN_DUE_AMOUNT,
                MIN_DUE_AMOUNT,
                0.into()
            ),
            Status::partial(MIN_DUE_AMOUNT, Cause::Overdue())
        );
        assert_eq!(
            check_liability::<Nls>(
                &spec,
                LEASE_AMOUNT,
                381.into(),
                0.into(),
                MIN_DUE_AMOUNT,
                0.into()
            ),
            Status::No(Zone::third(spec.third_liq_warn(), spec.max())),
        );
        assert_eq!(
            check_liability::<Nls>(
                &spec,
                LEASE_AMOUNT,
                382.into(),
                0.into(),
                MIN_DUE_AMOUNT,
                0.into()
            ),
            Status::No(Zone::third(spec.third_liq_warn(), spec.max())),
        );
        assert_eq!(
            check_liability::<Nls>(
                &spec,
                LEASE_AMOUNT,
                390.into(),
                0.into(),
                MIN_DUE_AMOUNT,
                0.into()
            ),
            Status::No(Zone::third(spec.third_liq_warn(), spec.max())),
        );
        assert_eq!(
            check_liability::<Nls>(
                &spec,
                LEASE_AMOUNT,
                391.into(),
                0.into(),
                MIN_DUE_AMOUNT,
                0.into()
            ),
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
        const BACK_TO_HEALTHY: Amount = 879;
        let spec = liability_with_max(max_ltv);

        assert_eq!(
            check_liability::<Nls>(
                &spec,
                LEASE_AMOUNT,
                880.into(),
                MIN_DUE_AMOUNT - 1.into(),
                MIN_DUE_AMOUNT,
                0.into()
            ),
            Status::No(Zone::third(spec.third_liq_warn(), spec.max())),
        );
        assert_eq!(
            check_liability::<Nls>(
                &spec,
                LEASE_AMOUNT,
                880.into(),
                MIN_DUE_AMOUNT,
                MIN_DUE_AMOUNT,
                0.into()
            ),
            Status::partial(MIN_DUE_AMOUNT, Cause::Overdue()),
        );
        assert_eq!(
            check_liability::<Nls>(
                &spec,
                LEASE_AMOUNT,
                881.into(),
                MIN_DUE_AMOUNT,
                MIN_DUE_AMOUNT,
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
                LEASE_AMOUNT,
                881.into(),
                (BACK_TO_HEALTHY - 1).into(),
                MIN_DUE_AMOUNT,
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
                LEASE_AMOUNT,
                881.into(),
                BACK_TO_HEALTHY.into(),
                MIN_DUE_AMOUNT,
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
                LEASE_AMOUNT,
                881.into(),
                (BACK_TO_HEALTHY + 1).into(),
                MIN_DUE_AMOUNT,
                0.into()
            ),
            Status::partial((BACK_TO_HEALTHY + 1).into(), Cause::Overdue()),
        );
        assert_eq!(
            check_liability::<Nls>(
                &spec,
                LEASE_AMOUNT,
                999.into(),
                997.into(),
                MIN_DUE_AMOUNT,
                0.into()
            ),
            Status::partial(
                998.into(),
                Cause::Liability {
                    ltv: max_ltv,
                    healthy_ltv: STEP
                }
            ),
        );
        assert_eq!(
            check_liability::<Nls>(
                &spec,
                LEASE_AMOUNT,
                LEASE_AMOUNT,
                1.into(),
                MIN_DUE_AMOUNT,
                0.into()
            ),
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
            check_liability::<Nls>(
                &spec,
                LEASE_AMOUNT,
                768.into(),
                765.into(),
                MIN_DUE_AMOUNT,
                0.into()
            ),
            Status::partial(
                765.into(),
                Cause::Liability {
                    ltv: max_ltv,
                    healthy_ltv: STEP
                }
            ),
        );
        assert_eq!(
            check_liability::<Nls>(
                &spec,
                LEASE_AMOUNT,
                768.into(),
                768.into(),
                MIN_DUE_AMOUNT,
                0.into()
            ),
            Status::partial(768.into(), Cause::Overdue()),
        );
        assert_eq!(
            check_liability::<Nls>(
                &spec,
                LEASE_AMOUNT,
                LEASE_AMOUNT,
                1.into(),
                MIN_DUE_AMOUNT,
                0.into()
            ),
            Status::full(Cause::Liability {
                ltv: max_ltv,
                healthy_ltv: STEP
            }),
        );
    }

    #[test]
    fn liquidate_full_min_asset() {
        let max_ltv = Percent::from_permille(573);
        let spec = liability_with_max(max_ltv);

        const BACK_TO_HEALTHY: Amount = 898;
        assert_eq!(
            check_liability::<Nls>(
                &spec,
                LEASE_AMOUNT,
                900.into(),
                BACK_TO_HEALTHY.into(),
                MIN_DUE_AMOUNT,
                LEASE_AMOUNT - BACK_TO_HEALTHY.into() - 1.into()
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
                LEASE_AMOUNT,
                900.into(),
                (BACK_TO_HEALTHY + 1).into(),
                MIN_DUE_AMOUNT,
                LEASE_AMOUNT - BACK_TO_HEALTHY.into() - 2.into()
            ),
            Status::partial((BACK_TO_HEALTHY + 1).into(), Cause::Overdue()),
        );
        assert_eq!(
            check_liability::<Nls>(
                &spec,
                LEASE_AMOUNT,
                572.into(),
                MIN_DUE_AMOUNT - 1.into(),
                MIN_DUE_AMOUNT,
                LEASE_AMOUNT
            ),
            Status::No(Zone::third(spec.third_liq_warn(), spec.max())),
        );
        assert_eq!(
            check_liability::<Nls>(
                &spec,
                LEASE_AMOUNT,
                900.into(),
                BACK_TO_HEALTHY.into(),
                MIN_DUE_AMOUNT,
                LEASE_AMOUNT - BACK_TO_HEALTHY.into()
            ),
            Status::full(Cause::Liability {
                ltv: max_ltv,
                healthy_ltv: STEP
            }),
        );
        assert_eq!(
            check_liability::<Nls>(
                &spec,
                LEASE_AMOUNT,
                900.into(),
                (BACK_TO_HEALTHY + 1).into(),
                MIN_DUE_AMOUNT,
                LEASE_AMOUNT - BACK_TO_HEALTHY.into() - 1.into()
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
