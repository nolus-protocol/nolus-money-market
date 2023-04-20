use serde::Serialize;

use finance::{coin::Coin, currency::Currency, percent::Percent};
use lpp::stub::lender::LppLender as LppLenderTrait;
use oracle::stub::Oracle as OracleTrait;
use platform::{batch::Batch, generate_ids};
use profit::stub::Profit as ProfitTrait;
use sdk::cosmwasm_std::{Addr, Timestamp};
use timealarms::stub::TimeAlarms as TimeAlarmsTrait;

use crate::{lease::Lease, loan::RepayReceipt};

mod alarm;

impl<Lpn, Asset, Lpp, Profit, TimeAlarms, Oracle> Lease<Lpn, Asset, Lpp, Profit, TimeAlarms, Oracle>
where
    Lpn: Currency + Serialize,
    Lpp: LppLenderTrait<Lpn>,
    TimeAlarms: TimeAlarmsTrait,
    Oracle: OracleTrait<Lpn>,
    Profit: ProfitTrait,
    Asset: Currency + Serialize,
{
    fn act_on_overdue(
        &mut self,
        now: Timestamp,
        ltv: Percent,
        _: Coin<Asset>,
        overdue: Coin<Asset>,
    ) -> Status<Asset> {
        if self.loan.grace_period_end() <= now {
            self.liquidate_on_interest_overdue(overdue)
        } else {
            self.handle_warnings(ltv)
        }
    }

    fn act_on_liability(
        &mut self,
        _now: Timestamp,
        ltv: Percent,
        total_due: Coin<Asset>,
        _: Coin<Asset>,
    ) -> Status<Asset> {
        if self.liability.max_percent() <= ltv {
            self.liquidate_on_liability(total_due)
        } else {
            self.handle_warnings(ltv)
        }
    }

    fn handle_warnings(&self, liability: Percent) -> Status<Asset> {
        debug_assert!(liability < self.liability.max_percent());

        if liability < self.liability.first_liq_warn_percent() {
            return Status::None;
        }

        let (ltv, level) = if self.liability.third_liq_warn_percent() <= liability {
            (self.liability.third_liq_warn_percent(), WarningLevel::Third)
        } else if self.liability.second_liq_warn_percent() <= liability {
            (
                self.liability.second_liq_warn_percent(),
                WarningLevel::Second,
            )
        } else {
            debug_assert!(self.liability.first_liq_warn_percent() <= liability);
            (self.liability.first_liq_warn_percent(), WarningLevel::First)
        };

        Status::Warning { ltv, level }
    }

    fn liquidate_on_liability(&mut self, total_due: Coin<Asset>) -> Status<Asset> {
        let liquidation = self.liability.amount_to_liquidate(self.amount, total_due);

        self.liquidate(
            Cause::Liability {
                ltv: self.liability.max_percent(),
                healthy_ltv: self.liability.healthy_percent(),
            },
            liquidation,
        )
    }

    fn liquidate_on_interest_overdue(&mut self, overdue: Coin<Asset>) -> Status<Asset> {
        self.liquidate(Cause::Overdue(), overdue)
    }

    fn liquidate(&mut self, cause: Cause, liquidation: Coin<Asset>) -> Status<Asset> {
        // let receipt = self.no_reschedule_repay(liquidation_lpn, now)?;

        // let liquidation_info = LiquidationInfo {
        //     cause,
        //     lease: self.addr.clone(),
        //     receipt,
        // };

        // TODO liquidate fully if the remaining value, lease_lpn - liquidation_lpn < 100
        if self.amount <= liquidation {
            Status::FullLiquidation(cause)
        } else {
            Status::PartialLiquidation {
                amount: liquidation,
                cause,
            }
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

#[cfg_attr(test, derive(Debug, Eq, PartialEq))]
pub(crate) enum Status<Asset>
where
    Asset: Currency,
{
    None,
    Warning { ltv: Percent, level: WarningLevel },
    PartialLiquidation { amount: Coin<Asset>, cause: Cause },
    FullLiquidation(Cause),
}

#[cfg_attr(test, derive(Debug, Eq, PartialEq))]
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

generate_ids! {
    pub(crate) WarningLevel as u8 {
        First = 1,
        Second = 2,
        Third = 3,
    }
}

impl WarningLevel {
    pub fn to_uint(self) -> u8 {
        self.into()
    }
}

#[cfg(test)]
mod tests {
    use finance::{coin::Amount, fraction::Fraction, percent::Percent};
    use lpp::msg::LoanResponse;
    use sdk::cosmwasm_std::{Addr, Timestamp};

    use crate::{
        lease::{
            tests::{coin, loan, lpn_coin, open_lease, LEASE_START},
            Status, WarningLevel,
        },
        loan::LiabilityStatus,
    };

    use super::Cause;

    #[test]
    fn warnings_none() {
        let interest_rate = Percent::from_permille(50);
        // LPP loan
        let loan = LoanResponse {
            principal_due: lpn_coin(500),
            annual_interest_rate: interest_rate,
            interest_paid: Timestamp::from_nanos(0),
        };

        let lease_addr = Addr::unchecked("lease");
        let lease = open_lease(
            lease_addr,
            10.into(),
            Some(loan),
            Addr::unchecked(String::new()),
            Addr::unchecked(String::new()),
            Addr::unchecked(String::new()),
        );

        assert_eq!(
            lease.handle_warnings(Percent::from_percent(60)),
            Status::None,
        );
    }

    #[test]
    fn warnings_first() {
        let interest_rate = Percent::from_permille(50);
        // LPP loan
        let loan = LoanResponse {
            principal_due: lpn_coin(500),
            annual_interest_rate: interest_rate,
            interest_paid: Timestamp::from_nanos(0),
        };

        let lease_addr = Addr::unchecked("lease");
        let lease = open_lease(
            lease_addr,
            10.into(),
            Some(loan),
            Addr::unchecked(String::new()),
            Addr::unchecked(String::new()),
            Addr::unchecked(String::new()),
        );

        assert_eq!(
            lease.handle_warnings(lease.liability.first_liq_warn_percent()),
            Status::Warning {
                ltv: lease.liability.first_liq_warn_percent(),
                level: WarningLevel::First,
            }
        );
    }

    #[test]
    fn warnings_second() {
        let interest_rate = Percent::from_permille(50);
        // LPP loan
        let loan = LoanResponse {
            principal_due: lpn_coin(500),
            annual_interest_rate: interest_rate,
            interest_paid: Timestamp::from_nanos(0),
        };

        let lease_addr = Addr::unchecked("lease");
        let lease = open_lease(
            lease_addr,
            10.into(),
            Some(loan),
            Addr::unchecked(String::new()),
            Addr::unchecked(String::new()),
            Addr::unchecked(String::new()),
        );

        assert_eq!(
            lease.handle_warnings(lease.liability.second_liq_warn_percent()),
            Status::Warning {
                ltv: lease.liability.second_liq_warn_percent(),
                level: WarningLevel::Second,
            }
        );
    }

    #[test]
    fn warnings_third() {
        let lease_addr = Addr::unchecked("lease");
        let lease = open_lease(
            lease_addr,
            10.into(),
            Some(loan()),
            Addr::unchecked(String::new()),
            Addr::unchecked(String::new()),
            Addr::unchecked(String::new()),
        );

        assert_eq!(
            lease.handle_warnings(lease.liability.third_liq_warn_percent()),
            Status::Warning {
                ltv: lease.liability.third_liq_warn_percent(),
                level: WarningLevel::Third,
            }
        );
    }

    #[test]
    fn liability() {
        // LPP loan
        let loan = LoanResponse {
            principal_due: lpn_coin(500),
            annual_interest_rate: Percent::from_permille(50),
            interest_paid: LEASE_START,
        };

        let lease_addr = Addr::unchecked("lease");
        let lease = open_lease(
            lease_addr,
            10.into(),
            Some(loan),
            Addr::unchecked(String::new()),
            Addr::unchecked(String::new()),
            Addr::unchecked(String::new()),
        );
        // lease.repay();
        // 100 days period
        // 100 interest due
        // let interest_due = 100.into();

        assert_eq!(
            lease
                .loan
                .liability_status(LEASE_START, Addr::unchecked(String::new()), lpn_coin(1000))
                .unwrap(),
            LiabilityStatus {
                ltv: Percent::from_percent(50),
                total_lpn: lpn_coin(500),
                overdue_lpn: lpn_coin(0),
            }
        );
    }

    #[test]
    fn liquidate_partial() {
        let lease_amount = coin(100);
        let loan_amount_lpn = lpn_coin(500);
        let interest_rate = Percent::from_percent(114);
        // LPP loan
        let loan = LoanResponse {
            principal_due: loan_amount_lpn,
            annual_interest_rate: interest_rate,
            interest_paid: LEASE_START,
        };

        let lease_addr = Addr::unchecked("lease");
        let mut lease = open_lease(
            lease_addr,
            lease_amount,
            Some(loan),
            Addr::unchecked(String::new()),
            Addr::unchecked(String::new()),
            Addr::unchecked(String::new()),
        );

        let total_due =
            (lease.liability.max_percent() + Percent::from_percent(10)).of(lease_amount);
        let exp_liquidate = coin(66);
        assert!(
            Amount::from(
                lease
                    .liability
                    .healthy_percent()
                    .of(lease_amount - exp_liquidate)
            )
            .abs_diff(Amount::from(total_due - exp_liquidate))
                <= 1
        );
        assert_eq!(
            lease.liquidate_on_liability(dbg!(total_due)),
            Status::PartialLiquidation {
                amount: exp_liquidate,
                cause: Cause::Liability {
                    ltv: lease.liability.max_percent(),
                    healthy_ltv: lease.liability.healthy_percent()
                }
            }
        );
    }

    #[test]
    fn liquidate_full() {
        let lease_amount = coin(100);
        let loan_amount_lpn = lpn_coin(500);
        let interest_rate = Percent::from_percent(242);
        // LPP loan
        let loan = LoanResponse {
            principal_due: loan_amount_lpn,
            annual_interest_rate: interest_rate,
            interest_paid: LEASE_START,
        };
        let lease_addr = Addr::unchecked("lease");
        let mut lease = open_lease(
            lease_addr,
            lease_amount,
            Some(loan),
            Addr::unchecked(String::new()),
            Addr::unchecked(String::new()),
            Addr::unchecked(String::new()),
        );

        let healthy_due = lease.liability.healthy_percent().of(lease_amount);

        assert_eq!(
            lease.liquidate_on_liability(healthy_due + lease_amount),
            Status::FullLiquidation(Cause::Liability {
                ltv: lease.liability.max_percent(),
                healthy_ltv: lease.liability.healthy_percent()
            })
        );
    }
}
