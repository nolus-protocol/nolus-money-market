use serde::Serialize;

use finance::{
    coin::Coin, currency::Currency, liability::Liability, percent::Percent, price, zero::Zero,
};
use lpp::stub::lender::LppLender as LppLenderTrait;
use oracle::stub::Oracle as OracleTrait;
use platform::batch::Batch;
use profit::stub::Profit as ProfitTrait;
use sdk::cosmwasm_std::{Addr, Timestamp};
use timealarms::stub::TimeAlarms as TimeAlarmsTrait;

use crate::{
    error::ContractResult,
    loan::{LiabilityStatus, RepayReceipt},
};

use super::Lease;

impl<Lpn, Asset, Lpp, Profit, TimeAlarms, Oracle> Lease<Lpn, Asset, Lpp, Profit, TimeAlarms, Oracle>
where
    Lpn: Currency + Serialize,
    Lpp: LppLenderTrait<Lpn>,
    TimeAlarms: TimeAlarmsTrait,
    Oracle: OracleTrait<Lpn>,
    Profit: ProfitTrait,
    Asset: Currency + Serialize,
{
    pub(crate) fn liquidation_status(&self, now: Timestamp) -> ContractResult<Status<Asset>> {
        let price_to_asset = self.price_of_lease_currency()?.inv();

        let LiabilityStatus {
            total: total_due,
            previous_interest,
        } = self.loan.liability_status(now, self.addr.clone())?;

        let overdue = if self.loan.grace_period_end() <= now {
            previous_interest
        } else {
            Coin::ZERO
        };

        Ok(check_liability(
            &self.liability,
            self.amount,
            price::total(total_due, price_to_asset),
            price::total(overdue, price_to_asset),
        ))
    }
}

fn check_liability<Asset>(
    spec: &Liability,
    asset: Coin<Asset>,
    total_due: Coin<Asset>,
    overdue: Coin<Asset>,
) -> Status<Asset>
where
    Asset: Currency,
{
    let liquidation = may_ask_liquidation_liability(spec, asset, total_due)
        .max(may_ask_liquidation_overdue(asset, overdue));
    if liquidation == Status::None {
        may_emit_warning(spec, asset, total_due)
    } else {
        liquidation
    }
}

fn may_emit_warning<Asset>(
    spec: &Liability,
    asset: Coin<Asset>,
    total_due: Coin<Asset>,
) -> Status<Asset>
where
    Asset: Currency,
{
    let ltv = Percent::from_ratio(total_due, asset);
    debug_assert!(ltv < spec.max_percent());

    if ltv < spec.first_liq_warn_percent() {
        return Status::None;
    }

    let level = if spec.third_liq_warn_percent() <= ltv {
        WarningLevel::third(spec)
    } else if spec.second_liq_warn_percent() <= ltv {
        WarningLevel::second(spec)
    } else {
        debug_assert!(spec.first_liq_warn_percent() <= ltv);
        WarningLevel::first(spec)
    };

    Status::Warning(level)
}

fn may_ask_liquidation_liability<Asset>(
    spec: &Liability,
    asset: Coin<Asset>,
    total_due: Coin<Asset>,
) -> Status<Asset>
where
    Asset: Currency,
{
    may_ask_liquidation(
        asset,
        Cause::Liability {
            ltv: spec.max_percent(),
            healthy_ltv: spec.healthy_percent(),
        },
        spec.amount_to_liquidate(asset, total_due),
    )
}

fn may_ask_liquidation_overdue<Asset>(asset: Coin<Asset>, overdue: Coin<Asset>) -> Status<Asset>
where
    Asset: Currency,
{
    may_ask_liquidation(asset, Cause::Overdue(), overdue)
}

fn may_ask_liquidation<Asset>(
    asset: Coin<Asset>,
    cause: Cause,
    liquidation: Coin<Asset>,
) -> Status<Asset>
where
    Asset: Currency,
{
    // TODO liquidate fully if the remaining value, lease_lpn - liquidation_lpn < 100
    if liquidation.is_zero() {
        Status::None
    } else if asset <= liquidation {
        Status::FullLiquidation(cause)
    } else {
        Status::PartialLiquidation {
            amount: liquidation,
            cause,
        }
    }
}

pub(crate) struct OnAlarmResult<Lpn>
where
    Lpn: Currency,
{
    pub batch: Batch,
    pub liquidation_status: Status<Lpn>,
}

#[derive(Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
#[cfg_attr(test, derive(Debug))]
pub(crate) enum Status<Asset>
where
    Asset: Currency,
{
    None,
    Warning(WarningLevel),
    PartialLiquidation { amount: Coin<Asset>, cause: Cause },
    FullLiquidation(Cause),
}

#[derive(Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
#[cfg_attr(test, derive(Debug))]
pub(crate) enum Cause {
    Overdue(),
    Liability { ltv: Percent, healthy_ltv: Percent },
}

pub(crate) trait LeaseInfo {
    type Asset: Currency;

    fn lease(&self) -> &Addr;
    fn customer(&self) -> &Addr;
}

#[cfg_attr(test, derive(Debug, Eq, PartialEq))]
pub(crate) struct LiquidationInfo<Lpn>
where
    Lpn: Currency,
{
    pub cause: Cause,
    pub lease: Addr,
    pub receipt: RepayReceipt<Lpn>,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(test, derive(Debug))]
pub(crate) enum WarningLevel {
    First(Percent),
    Second(Percent),
    Third(Percent),
}

impl WarningLevel {
    pub fn first(spec: &Liability) -> Self {
        Self::First(spec.first_liq_warn_percent())
    }

    pub fn second(spec: &Liability) -> Self {
        Self::Second(spec.second_liq_warn_percent())
    }

    pub fn third(spec: &Liability) -> Self {
        Self::Third(spec.third_liq_warn_percent())
    }

    pub fn ltv(&self) -> Percent {
        *match self {
            Self::First(ltv) | Self::Second(ltv) | Self::Third(ltv) => ltv,
        }
    }

    pub fn ordinal(self) -> u8 {
        match self {
            Self::First(_) => 1,
            Self::Second(_) => 2,
            Self::Third(_) => 3,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::lease::{liquidation::check_liability, Status, WarningLevel};
    use currency::lease::Atom;
    use finance::{duration::Duration, liability::Liability, percent::Percent};

    use super::Cause;

    #[test]
    fn warnings_none() {
        let warn_ltv = Percent::from_percent(51);
        let spec = liability_with_first(warn_ltv);
        assert_eq!(
            check_liability::<Atom>(&spec, 100.into(), 0.into(), 0.into()),
            Status::None,
        );
        assert_eq!(
            check_liability::<Atom>(&spec, 100.into(), 49.into(), 0.into()),
            Status::None,
        );
        assert_eq!(
            check_liability::<Atom>(&spec, 100.into(), 50.into(), 0.into()),
            Status::None,
        );
        assert_eq!(
            check_liability::<Atom>(&spec, 1000.into(), 505.into(), 1.into()),
            Status::PartialLiquidation {
                amount: 1.into(),
                cause: Cause::Overdue()
            },
        );
        assert_eq!(
            check_liability::<Atom>(&spec, 1000.into(), 509.into(), 0.into()),
            Status::None,
        );
        assert_eq!(
            check_liability::<Atom>(&spec, 1000.into(), 510.into(), 0.into()),
            Status::Warning(WarningLevel::first(&spec))
        );
        assert_eq!(
            check_liability::<Atom>(&spec, 1000.into(), 510.into(), 1.into()),
            Status::PartialLiquidation {
                amount: 1.into(),
                cause: Cause::Overdue()
            },
        );
    }

    #[test]
    fn warnings_first() {
        let spec = liability_with_first(Percent::from_permille(712));

        assert_eq!(
            check_liability::<Atom>(&spec, 1000.into(), 711.into(), 0.into()),
            Status::None,
        );
        assert_eq!(
            check_liability::<Atom>(&spec, 1000.into(), 712.into(), 0.into()),
            Status::Warning(WarningLevel::first(&spec))
        );
        assert_eq!(
            check_liability::<Atom>(&spec, 1000.into(), 712.into(), 1.into()),
            Status::PartialLiquidation {
                amount: 1.into(),
                cause: Cause::Overdue()
            }
        );
        assert_eq!(
            check_liability::<Atom>(&spec, 1000.into(), 715.into(), 0.into()),
            Status::Warning(WarningLevel::first(&spec)),
        );
        assert_eq!(
            check_liability::<Atom>(&spec, 1000.into(), 721.into(), 0.into()),
            Status::Warning(WarningLevel::first(&spec)),
        );
        assert_eq!(
            check_liability::<Atom>(&spec, 1000.into(), 722.into(), 0.into()),
            Status::Warning(WarningLevel::second(&spec)),
        );
    }

    #[test]
    fn warnings_second() {
        let spec = liability_with_second(Percent::from_permille(123));

        assert_eq!(
            check_liability::<Atom>(&spec, 1000.into(), 122.into(), 0.into()),
            Status::Warning(WarningLevel::first(&spec)),
        );
        assert_eq!(
            check_liability::<Atom>(&spec, 1000.into(), 123.into(), 0.into()),
            Status::Warning(WarningLevel::second(&spec)),
        );
        assert_eq!(
            check_liability::<Atom>(&spec, 1000.into(), 124.into(), 0.into()),
            Status::Warning(WarningLevel::second(&spec)),
        );
        assert_eq!(
            check_liability::<Atom>(&spec, 1000.into(), 128.into(), 1.into()),
            Status::PartialLiquidation {
                amount: 1.into(),
                cause: Cause::Overdue()
            }
        );
        assert_eq!(
            check_liability::<Atom>(&spec, 1000.into(), 132.into(), 0.into()),
            Status::Warning(WarningLevel::second(&spec)),
        );
        assert_eq!(
            check_liability::<Atom>(&spec, 1000.into(), 133.into(), 0.into()),
            Status::Warning(WarningLevel::third(&spec)),
        );
    }

    #[test]
    fn warnings_third() {
        let warn_third_ltv = Percent::from_permille(381);
        let max_ltv = warn_third_ltv + STEP;
        let spec = liability_with_third(warn_third_ltv);

        assert_eq!(
            check_liability::<Atom>(&spec, 1000.into(), 380.into(), 0.into()),
            Status::Warning(WarningLevel::second(&spec)),
        );
        assert_eq!(
            check_liability::<Atom>(&spec, 1000.into(), 381.into(), 1.into()),
            Status::PartialLiquidation {
                amount: 1.into(),
                cause: Cause::Overdue()
            }
        );
        assert_eq!(
            check_liability::<Atom>(&spec, 1000.into(), 381.into(), 0.into()),
            Status::Warning(WarningLevel::third(&spec)),
        );
        assert_eq!(
            check_liability::<Atom>(&spec, 1000.into(), 382.into(), 0.into()),
            Status::Warning(WarningLevel::third(&spec)),
        );
        assert_eq!(
            check_liability::<Atom>(&spec, 1000.into(), 390.into(), 0.into()),
            Status::Warning(WarningLevel::third(&spec)),
        );
        assert_eq!(
            check_liability::<Atom>(&spec, 1000.into(), 391.into(), 0.into()),
            Status::PartialLiquidation {
                amount: 384.into(),
                cause: Cause::Liability {
                    ltv: max_ltv,
                    healthy_ltv: STEP
                }
            },
        );
    }

    #[test]
    fn liquidate_partial() {
        let max_ltv = Percent::from_permille(881);
        let spec = liability_with_max(max_ltv);

        assert_eq!(
            check_liability::<Atom>(&spec, 1000.into(), 880.into(), 0.into()),
            Status::Warning(WarningLevel::third(&spec)),
        );
        assert_eq!(
            check_liability::<Atom>(&spec, 1000.into(), 880.into(), 1.into()),
            Status::PartialLiquidation {
                amount: 1.into(),
                cause: Cause::Overdue()
            },
        );
        assert_eq!(
            check_liability::<Atom>(&spec, 1000.into(), 881.into(), 0.into()),
            Status::PartialLiquidation {
                amount: 879.into(),
                cause: Cause::Liability {
                    ltv: max_ltv,
                    healthy_ltv: STEP
                }
            },
        );
        assert_eq!(
            check_liability::<Atom>(&spec, 1000.into(), 881.into(), 878.into()),
            Status::PartialLiquidation {
                amount: 879.into(),
                cause: Cause::Liability {
                    ltv: max_ltv,
                    healthy_ltv: STEP
                }
            },
        );
        assert_eq!(
            check_liability::<Atom>(&spec, 1000.into(), 881.into(), 879.into()),
            Status::PartialLiquidation {
                amount: 879.into(),
                cause: Cause::Liability {
                    ltv: max_ltv,
                    healthy_ltv: STEP
                }
            },
        );
        assert_eq!(
            check_liability::<Atom>(&spec, 1000.into(), 881.into(), 880.into()),
            Status::PartialLiquidation {
                amount: 880.into(),
                cause: Cause::Overdue()
            },
        );
        assert_eq!(
            check_liability::<Atom>(&spec, 1000.into(), 999.into(), 997.into()),
            Status::PartialLiquidation {
                amount: 998.into(),
                cause: Cause::Liability {
                    ltv: max_ltv,
                    healthy_ltv: STEP
                }
            },
        );
        assert_eq!(
            check_liability::<Atom>(&spec, 1000.into(), 1000.into(), 1.into()),
            Status::FullLiquidation(Cause::Liability {
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
            check_liability::<Atom>(&spec, 1000.into(), 768.into(), 765.into()),
            Status::PartialLiquidation {
                amount: 765.into(),
                cause: Cause::Liability {
                    ltv: max_ltv,
                    healthy_ltv: STEP
                }
            },
        );
        assert_eq!(
            check_liability::<Atom>(&spec, 1000.into(), 768.into(), 766.into()),
            Status::PartialLiquidation {
                amount: 766.into(),
                cause: Cause::Overdue()
            },
        );
        assert_eq!(
            check_liability::<Atom>(&spec, 1000.into(), 1000.into(), 1.into()),
            Status::FullLiquidation(Cause::Liability {
                ltv: max_ltv,
                healthy_ltv: STEP
            }),
        );
        assert_eq!(
            check_liability::<Atom>(&spec, 1000.into(), 1000.into(), 1000.into()),
            Status::FullLiquidation(Cause::Liability {
                ltv: max_ltv,
                healthy_ltv: STEP
            }),
        );
        assert_eq!(
            check_liability::<Atom>(&spec, 1000.into(), 999.into(), 1000.into()),
            Status::FullLiquidation(Cause::Overdue()),
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
    use finance::percent::Percent;

    use crate::lease::{Cause, WarningLevel};

    use super::Status;

    #[test]
    fn ord() {
        assert!(
            Status::<Cro>::None < Status::Warning(WarningLevel::First(Percent::from_permille(1)))
        );
        assert!(
            Status::<Cro>::Warning(WarningLevel::First(Percent::from_permille(1)))
                < Status::Warning(WarningLevel::Second(Percent::from_permille(1)))
        );
        assert!(
            Status::<Cro>::Warning(WarningLevel::First(Percent::from_permille(1)))
                < Status::Warning(WarningLevel::First(Percent::from_permille(2)))
        );
        // NB! the contract is not respected
        assert!(
            Status::Warning(WarningLevel::First(Percent::from_permille(2)))
                < Status::<Cro>::Warning(WarningLevel::Second(Percent::from_permille(1)))
        );
        assert!(
            Status::Warning(WarningLevel::Third(Percent::from_permille(100)))
                < Status::<Cro>::PartialLiquidation {
                    amount: 1.into(),
                    cause: Cause::Overdue()
                }
        );
        assert!(
            Status::<Cro>::PartialLiquidation {
                amount: 1.into(),
                cause: Cause::Overdue()
            } < Status::<Cro>::PartialLiquidation {
                amount: 1.into(),
                cause: Cause::Liability {
                    ltv: Percent::from_permille(1),
                    healthy_ltv: Percent::from_permille(1)
                }
            }
        );
        assert!(
            Status::<Cro>::PartialLiquidation {
                amount: 1.into(),
                cause: Cause::Overdue()
            } < Status::<Cro>::PartialLiquidation {
                amount: 2.into(),
                cause: Cause::Overdue()
            }
        );
        assert!(
            Status::<Cro>::PartialLiquidation {
                amount: 1.into(),
                cause: Cause::Liability {
                    ltv: Percent::from_permille(1),
                    healthy_ltv: Percent::from_permille(1)
                }
            } < Status::<Cro>::PartialLiquidation {
                amount: 2.into(),
                cause: Cause::Liability {
                    ltv: Percent::from_permille(1),
                    healthy_ltv: Percent::from_permille(1)
                }
            }
        );
        assert!(
            Status::<Cro>::PartialLiquidation {
                amount: 1.into(),
                cause: Cause::Liability {
                    ltv: Percent::from_permille(1),
                    healthy_ltv: Percent::from_permille(1)
                }
            } < Status::<Cro>::PartialLiquidation {
                amount: 1.into(),
                cause: Cause::Liability {
                    ltv: Percent::from_permille(1),
                    healthy_ltv: Percent::from_permille(2)
                }
            }
        );
        assert!(
            Status::<Cro>::PartialLiquidation {
                amount: 1.into(),
                cause: Cause::Liability {
                    ltv: Percent::from_permille(1),
                    healthy_ltv: Percent::from_permille(1)
                }
            } < Status::<Cro>::FullLiquidation(Cause::Liability {
                ltv: Percent::from_permille(1),
                healthy_ltv: Percent::from_permille(2)
            })
        );
        assert!(
            Status::<Cro>::FullLiquidation(Cause::Liability {
                ltv: Percent::from_permille(1),
                healthy_ltv: Percent::from_permille(1)
            }) < Status::<Cro>::FullLiquidation(Cause::Liability {
                ltv: Percent::from_permille(1),
                healthy_ltv: Percent::from_permille(2)
            })
        );
        assert!(
            Status::<Cro>::FullLiquidation(Cause::Overdue())
                < Status::<Cro>::FullLiquidation(Cause::Liability {
                    ltv: Percent::from_permille(1),
                    healthy_ltv: Percent::from_permille(1)
                })
        );
    }
}
