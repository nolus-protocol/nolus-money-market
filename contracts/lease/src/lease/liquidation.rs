use serde::{Deserialize, Serialize};

use finance::{
    coin::Coin,
    currency::Currency,
    liability::{Liability, Zone},
    percent::Percent,
    price::{self, Price},
    zero::Zero,
};
use lpp::stub::loan::LppLoan as LppLoanTrait;
use oracle::stub::Oracle as OracleTrait;
use sdk::cosmwasm_std::Timestamp;

use crate::{error::ContractResult, loan::LiabilityStatus};

use super::Lease;

#[derive(Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
#[cfg_attr(test, derive(Debug))]
pub(crate) enum Status<Asset>
where
    Asset: Currency,
{
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
pub(crate) enum Liquidation<Asset>
where
    Asset: Currency,
{
    Partial { amount: Coin<Asset>, cause: Cause },
    Full(Cause),
}

#[derive(Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[cfg_attr(test, derive(Debug))]
pub(crate) enum Cause {
    Overdue(),
    Liability { ltv: Percent, healthy_ltv: Percent },
}

impl<Lpn, Asset, Lpp, Oracle> Lease<Lpn, Asset, Lpp, Oracle>
where
    Lpn: Currency + Serialize,
    Lpp: LppLoanTrait<Lpn>,
    Oracle: OracleTrait<Lpn>,
    Asset: Currency + Serialize,
{
    const MIN_ASSET_AMOUNT_BEFORE_LIQUIDATION: Coin<Lpn> = Coin::new(15_000_000); // TODO issue #50

    pub(crate) fn liquidation_status(&self, now: Timestamp) -> ContractResult<Status<Asset>> {
        let price_in_asset = self.price_of_lease_currency()?.inv();

        let LiabilityStatus {
            total: total_due,
            previous_interest,
        } = self.loan.liability_status(now)?;

        let overdue = if self.loan.grace_period_end() <= now {
            previous_interest
        } else {
            Coin::ZERO
        };

        let status = check_liability(
            &self.liability,
            self.amount,
            price::total(total_due, price_in_asset),
            price::total(overdue, price_in_asset),
            price::total(Self::MIN_ASSET_AMOUNT_BEFORE_LIQUIDATION, price_in_asset),
        );
        #[cfg(debug_assertion)]
        debug_assert!(status.amount() <= self.amount());
        Ok(status)
    }

    fn price_of_lease_currency(&self) -> ContractResult<Price<Asset, Lpn>> {
        Ok(self.oracle.price_of::<Asset>()?)
    }
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

fn check_liability<Asset>(
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
    let ltv = Percent::from_ratio(total_due, asset);
    debug_assert!(ltv < spec.max());

    Status::No(spec.zone_of(ltv))
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
    use currency::lease::Atom;
    use finance::{
        coin::Amount,
        duration::Duration,
        liability::{Liability, Zone},
        percent::Percent,
    };

    use super::{super::Status, check_liability, Cause};

    #[test]
    fn warnings_none() {
        let warn_ltv = Percent::from_percent(51);
        let spec = liability_with_first(warn_ltv);
        assert_eq!(
            check_liability::<Atom>(&spec, 100.into(), 0.into(), 0.into(), 0.into()),
            Status::No(Zone::no_warnings(spec.first_liq_warn())),
        );
        assert_eq!(
            check_liability::<Atom>(&spec, 100.into(), 49.into(), 0.into(), 0.into()),
            Status::No(Zone::no_warnings(spec.first_liq_warn())),
        );
        assert_eq!(
            check_liability::<Atom>(&spec, 100.into(), 50.into(), 0.into(), 0.into()),
            Status::No(Zone::no_warnings(spec.first_liq_warn())),
        );
        assert_eq!(
            check_liability::<Atom>(&spec, 1000.into(), 505.into(), 1.into(), 0.into()),
            Status::partial(1.into(), Cause::Overdue()),
        );
        assert_eq!(
            check_liability::<Atom>(&spec, 1000.into(), 509.into(), 0.into(), 0.into()),
            Status::No(Zone::no_warnings(spec.first_liq_warn())),
        );
        assert_eq!(
            check_liability::<Atom>(&spec, 1000.into(), 510.into(), 0.into(), 0.into()),
            Status::No(Zone::first(spec.first_liq_warn(), spec.second_liq_warn())),
        );
        assert_eq!(
            check_liability::<Atom>(&spec, 1000.into(), 510.into(), 1.into(), 0.into()),
            Status::partial(1.into(), Cause::Overdue()),
        );
    }

    #[test]
    fn warnings_first() {
        let spec = liability_with_first(Percent::from_permille(712));

        assert_eq!(
            check_liability::<Atom>(&spec, 1000.into(), 711.into(), 0.into(), 0.into()),
            Status::No(Zone::no_warnings(spec.first_liq_warn())),
        );
        assert_eq!(
            check_liability::<Atom>(&spec, 1000.into(), 712.into(), 0.into(), 0.into()),
            Status::No(Zone::first(spec.first_liq_warn(), spec.second_liq_warn())),
        );
        assert_eq!(
            check_liability::<Atom>(&spec, 1000.into(), 712.into(), 1.into(), 0.into()),
            Status::partial(1.into(), Cause::Overdue())
        );
        assert_eq!(
            check_liability::<Atom>(&spec, 1000.into(), 715.into(), 0.into(), 0.into()),
            Status::No(Zone::first(spec.first_liq_warn(), spec.second_liq_warn())),
        );
        assert_eq!(
            check_liability::<Atom>(&spec, 1000.into(), 721.into(), 0.into(), 0.into()),
            Status::No(Zone::first(spec.first_liq_warn(), spec.second_liq_warn())),
        );
        assert_eq!(
            check_liability::<Atom>(&spec, 1000.into(), 722.into(), 0.into(), 0.into()),
            Status::No(Zone::second(spec.second_liq_warn(), spec.third_liq_warn())),
        );
    }

    #[test]
    fn warnings_second() {
        let spec = liability_with_second(Percent::from_permille(123));

        assert_eq!(
            check_liability::<Atom>(&spec, 1000.into(), 122.into(), 0.into(), 0.into()),
            Status::No(Zone::first(spec.first_liq_warn(), spec.second_liq_warn())),
        );
        assert_eq!(
            check_liability::<Atom>(&spec, 1000.into(), 123.into(), 0.into(), 0.into()),
            Status::No(Zone::second(spec.second_liq_warn(), spec.third_liq_warn())),
        );
        assert_eq!(
            check_liability::<Atom>(&spec, 1000.into(), 124.into(), 0.into(), 0.into()),
            Status::No(Zone::second(spec.second_liq_warn(), spec.third_liq_warn())),
        );
        assert_eq!(
            check_liability::<Atom>(&spec, 1000.into(), 128.into(), 1.into(), 0.into()),
            Status::partial(1.into(), Cause::Overdue())
        );
        assert_eq!(
            check_liability::<Atom>(&spec, 1000.into(), 132.into(), 0.into(), 0.into()),
            Status::No(Zone::second(spec.second_liq_warn(), spec.third_liq_warn())),
        );
        assert_eq!(
            check_liability::<Atom>(&spec, 1000.into(), 133.into(), 0.into(), 0.into()),
            Status::No(Zone::third(spec.third_liq_warn(), spec.max())),
        );
    }

    #[test]
    fn warnings_third() {
        let warn_third_ltv = Percent::from_permille(381);
        let max_ltv = warn_third_ltv + STEP;
        let spec = liability_with_third(warn_third_ltv);

        assert_eq!(
            check_liability::<Atom>(&spec, 1000.into(), 380.into(), 0.into(), 0.into()),
            Status::No(Zone::second(spec.second_liq_warn(), spec.third_liq_warn())),
        );
        assert_eq!(
            check_liability::<Atom>(&spec, 1000.into(), 381.into(), 1.into(), 0.into()),
            Status::partial(1.into(), Cause::Overdue())
        );
        assert_eq!(
            check_liability::<Atom>(&spec, 1000.into(), 381.into(), 0.into(), 0.into()),
            Status::No(Zone::third(spec.third_liq_warn(), spec.max())),
        );
        assert_eq!(
            check_liability::<Atom>(&spec, 1000.into(), 382.into(), 0.into(), 0.into()),
            Status::No(Zone::third(spec.third_liq_warn(), spec.max())),
        );
        assert_eq!(
            check_liability::<Atom>(&spec, 1000.into(), 390.into(), 0.into(), 0.into()),
            Status::No(Zone::third(spec.third_liq_warn(), spec.max())),
        );
        assert_eq!(
            check_liability::<Atom>(&spec, 1000.into(), 391.into(), 0.into(), 0.into()),
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
            check_liability::<Atom>(&spec, 1000.into(), 880.into(), 0.into(), 0.into()),
            Status::No(Zone::third(spec.third_liq_warn(), spec.max())),
        );
        assert_eq!(
            check_liability::<Atom>(&spec, 1000.into(), 880.into(), 1.into(), 0.into()),
            Status::partial(1.into(), Cause::Overdue()),
        );
        assert_eq!(
            check_liability::<Atom>(&spec, 1000.into(), 881.into(), 0.into(), 0.into()),
            Status::partial(
                879.into(),
                Cause::Liability {
                    ltv: max_ltv,
                    healthy_ltv: STEP
                }
            ),
        );
        assert_eq!(
            check_liability::<Atom>(&spec, 1000.into(), 881.into(), 878.into(), 0.into()),
            Status::partial(
                879.into(),
                Cause::Liability {
                    ltv: max_ltv,
                    healthy_ltv: STEP
                }
            ),
        );
        assert_eq!(
            check_liability::<Atom>(&spec, 1000.into(), 881.into(), 879.into(), 0.into()),
            Status::partial(
                879.into(),
                Cause::Liability {
                    ltv: max_ltv,
                    healthy_ltv: STEP
                }
            ),
        );
        assert_eq!(
            check_liability::<Atom>(&spec, 1000.into(), 881.into(), 880.into(), 0.into()),
            Status::partial(880.into(), Cause::Overdue()),
        );
        assert_eq!(
            check_liability::<Atom>(&spec, 1000.into(), 999.into(), 997.into(), 0.into()),
            Status::partial(
                998.into(),
                Cause::Liability {
                    ltv: max_ltv,
                    healthy_ltv: STEP
                }
            ),
        );
        assert_eq!(
            check_liability::<Atom>(&spec, 1000.into(), 1000.into(), 1.into(), 0.into()),
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
            check_liability::<Atom>(&spec, 1000.into(), 768.into(), 765.into(), 0.into()),
            Status::partial(
                765.into(),
                Cause::Liability {
                    ltv: max_ltv,
                    healthy_ltv: STEP
                }
            ),
        );
        assert_eq!(
            check_liability::<Atom>(&spec, 1000.into(), 768.into(), 766.into(), 0.into()),
            Status::partial(766.into(), Cause::Overdue()),
        );
        assert_eq!(
            check_liability::<Atom>(&spec, 1000.into(), 1000.into(), 1.into(), 0.into()),
            Status::full(Cause::Liability {
                ltv: max_ltv,
                healthy_ltv: STEP
            }),
        );
        let back_to_healthy: Amount = spec.amount_to_liquidate(1000, 900);
        assert_eq!(
            check_liability::<Atom>(
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
            check_liability::<Atom>(
                &spec,
                1000.into(),
                900.into(),
                (back_to_healthy + 1).into(),
                (1000 - back_to_healthy - 2).into()
            ),
            Status::partial((back_to_healthy + 1).into(), Cause::Overdue()),
        );
        assert_eq!(
            check_liability::<Atom>(
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
            check_liability::<Atom>(
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
    use currency::lease::Cro;
    use finance::{liability::Zone, percent::Percent};

    use crate::lease::{liquidation::Liquidation, Cause};

    use super::Status;

    #[test]
    fn ord() {
        assert!(
            Status::<Cro>::No(Zone::no_warnings(Percent::from_permille(1)))
                < Status::No(Zone::first(
                    Percent::from_permille(1),
                    Percent::from_permille(2)
                ))
        );
        assert!(
            Status::<Cro>::No(Zone::first(
                Percent::from_permille(1),
                Percent::from_permille(2)
            )) < Status::No(Zone::second(
                Percent::from_permille(1),
                Percent::from_permille(2)
            ))
        );
        assert!(
            Status::<Cro>::No(Zone::first(
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
            )) < Status::<Cro>::No(Zone::second(
                Percent::from_permille(1),
                Percent::from_permille(2)
            ))
        );
        assert!(
            Status::No(Zone::third(
                Percent::from_permille(991),
                Percent::from_permille(1000)
            )) < Status::<Cro>::Liquidation(Liquidation::Partial {
                amount: 1.into(),
                cause: Cause::Overdue()
            })
        );
        assert!(
            Status::<Cro>::Liquidation(Liquidation::Partial {
                amount: 1.into(),
                cause: Cause::Overdue()
            }) < Status::<Cro>::Liquidation(Liquidation::Partial {
                amount: 1.into(),
                cause: Cause::Liability {
                    ltv: Percent::from_permille(1),
                    healthy_ltv: Percent::from_permille(1)
                }
            })
        );
        assert!(
            Status::<Cro>::Liquidation(Liquidation::Partial {
                amount: 1.into(),
                cause: Cause::Overdue()
            }) < Status::<Cro>::Liquidation(Liquidation::Partial {
                amount: 2.into(),
                cause: Cause::Overdue()
            })
        );
        assert!(
            Status::<Cro>::Liquidation(Liquidation::Partial {
                amount: 1.into(),
                cause: Cause::Liability {
                    ltv: Percent::from_permille(1),
                    healthy_ltv: Percent::from_permille(1)
                }
            }) < Status::<Cro>::Liquidation(Liquidation::Partial {
                amount: 2.into(),
                cause: Cause::Liability {
                    ltv: Percent::from_permille(1),
                    healthy_ltv: Percent::from_permille(1)
                }
            })
        );
        assert!(
            Status::<Cro>::partial(
                1.into(),
                Cause::Liability {
                    ltv: Percent::from_permille(1),
                    healthy_ltv: Percent::from_permille(1)
                }
            ) < Status::<Cro>::partial(
                1.into(),
                Cause::Liability {
                    ltv: Percent::from_permille(1),
                    healthy_ltv: Percent::from_permille(2)
                }
            )
        );
        assert!(
            Status::<Cro>::partial(
                1.into(),
                Cause::Liability {
                    ltv: Percent::from_permille(1),
                    healthy_ltv: Percent::from_permille(1)
                }
            ) < Status::<Cro>::full(Cause::Liability {
                ltv: Percent::from_permille(1),
                healthy_ltv: Percent::from_permille(2)
            })
        );
        assert!(
            Status::<Cro>::full(Cause::Liability {
                ltv: Percent::from_permille(1),
                healthy_ltv: Percent::from_permille(1)
            }) < Status::<Cro>::full(Cause::Liability {
                ltv: Percent::from_permille(1),
                healthy_ltv: Percent::from_permille(2)
            })
        );
        assert!(
            Status::<Cro>::full(Cause::Overdue())
                < Status::<Cro>::full(Cause::Liability {
                    ltv: Percent::from_permille(1),
                    healthy_ltv: Percent::from_permille(1)
                })
        );
    }
}
