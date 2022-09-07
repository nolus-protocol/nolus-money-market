use cosmwasm_std::{Addr, Timestamp};
use serde::Serialize;

use finance::{
    coin::Coin,
    currency::{Currency, SymbolOwned},
    fraction::Fraction,
    percent::{Percent, Units},
    price::{total, total_of, Price, PriceDTO},
    ratio::Rational,
};
use lpp::stub::Lpp as LppTrait;
use market_price_oracle::stub::Oracle as OracleTrait;
use marketprice::alarms::Alarm;
use platform::{bank::BankAccountView, batch::Batch, generate_ids};
use time_alarms::stub::TimeAlarms as TimeAlarmsTrait;

use crate::{error::ContractResult, lease::Lease};

use super::LeaseDTO;

impl<Lpn, Lpp, TimeAlarms, Oracle> Lease<Lpn, Lpp, TimeAlarms, Oracle>
where
    Lpn: Currency + Serialize,
    Lpp: LppTrait<Lpn>,
    TimeAlarms: TimeAlarmsTrait,
    Oracle: OracleTrait<Lpn>,
{
    pub(crate) fn on_price_alarm<B>(
        mut self,
        now: Timestamp,
        account: &B,
        lease: Addr,
        price: Price<Lpn, Lpn>,
    ) -> ContractResult<OnAlarmResult<Lpn>>
    where
        B: BankAccountView,
    {
        let status = self.on_alarm(now, account, lease, price)?;

        Ok(self.construct_on_alarm_result(status))
    }

    pub(crate) fn on_time_alarm<B>(
        mut self,
        now: Timestamp,
        account: &B,
        lease: Addr,
    ) -> ContractResult<OnAlarmResult<Lpn>>
    where
        B: BankAccountView,
    {
        let lease_amount = account.balance::<Lpn>()?;

        let status = if self.loan.grace_period_end() <= now {
            self.liquidate_on_interest_overdue(now, lease.clone(), lease_amount)?
        } else {
            Status::None
        };

        self.reschedule(lease, lease_amount, &now, &status)?;

        Ok(self.construct_on_alarm_result(status))
    }

    #[inline]
    pub(super) fn initial_alarm_schedule<A>(
        &mut self,
        lease: A,
        lease_amount: Coin<Lpn>,
        now: &Timestamp,
    ) -> ContractResult<()>
    where
        A: Into<Addr>,
    {
        self.reschedule(lease, lease_amount, now, &Status::None)
    }

    #[inline]
    pub(super) fn reschedule_on_repay<A>(
        &mut self,
        lease: A,
        lease_amount: Coin<Lpn>,
        now: &Timestamp,
    ) -> ContractResult<()>
        where
            A: Into<Addr> + Clone,
    {
        self.reschedule(
            lease.clone(),
            lease_amount,
            now,
            &self.handle_warnings(self.ltv(*now, lease, lease_amount)?.ltv),
        )
    }

    fn construct_on_alarm_result(self, liquidation_status: Status<Lpn>) -> OnAlarmResult<Lpn> {
        let (lease_dto, batch) = self.into_dto();

        OnAlarmResult {
            batch,
            lease_dto,
            liquidation_status,
        }
    }

    fn on_alarm<B>(
        &mut self,
        now: Timestamp,
        account: &B,
        lease: Addr,
        price: Price<Lpn, Lpn>,
    ) -> ContractResult<Status<Lpn>>
        where
            B: BankAccountView,
    {
        let lease_amount = account.balance::<Lpn>()?;

        let status = self.act_on_liability(now, lease.clone(), lease_amount, price)?;

        if !matches!(status, Status::FullLiquidation(_)) {
            self.reschedule(lease, lease_amount, &now, &status)?;
        }

        Ok(status)
    }

    fn liquidate_on_interest_overdue(
        &self,
        now: Timestamp,
        lease: Addr,
        lease_amount: Coin<Lpn>,
    ) -> ContractResult<Status<Lpn>> {
        let ltv = self.ltv(now, lease, lease_amount)?;

        let liquidation_amount = lease_amount.min(ltv.overdue);

        // TODO perform liquidation of asset

        let info = LeaseInfo {
            customer: self.customer.clone(),
            ltv: ltv.ltv,
            lease_asset: self.currency.clone(),
        };

        Ok(if liquidation_amount == lease_amount {
            Status::FullLiquidation(info)
        } else {
            Status::PartialLiquidation {
                _info: info,
                _healthy_ltv: self.liability.healthy_percent(),
                liquidation_amount,
            }
        })
    }

    fn act_on_liability(
        &self,
        now: Timestamp,
        lease: Addr,
        lease_amount: Coin<Lpn>,
        market_price: Price<Lpn, Lpn>,
    ) -> ContractResult<Status<Lpn>> {
        let LtvResult {
            ltv, liability_lpn, ..
        } = self.ltv(now, lease, lease_amount)?;

        let lease_lpn = total(lease_amount, market_price);

        Ok(if self.liability.max_percent() <= ltv {
            self.liquidate(
                self.customer.clone(),
                self.currency.clone(),
                lease_lpn,
                liability_lpn,
            )
        } else {
            self.handle_warnings(ltv)
        })
    }

    fn handle_warnings(&self, liability: Percent) -> Status<Lpn> {
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

        Status::Warning(
            LeaseInfo {
                customer: self.customer.clone(),
                ltv,
                lease_asset: self.currency.clone(),
            },
            level,
        )
    }

    fn ltv<A>(
        &self,
        now: Timestamp,
        lease: A,
        lease_lpn: Coin<Lpn>,
    ) -> ContractResult<LtvResult<Lpn>>
        where
            A: Into<Addr>,
    {
        self.loan
            .liability(now, lease)
            .map(|(liability_lpn, overdue)| LtvResult {
                ltv: Percent::from_ratio(liability_lpn, lease_lpn),
                liability_lpn,
                overdue,
            })
    }

    fn liquidate(
        &self,
        customer: Addr,
        lease_asset: SymbolOwned,
        lease_lpn: Coin<Lpn>,
        liability_lpn: Coin<Lpn>,
    ) -> Status<Lpn> {
        // from 'liability - liquidation = healthy% of (lease - liquidation)' follows
        // 'liquidation = 100% / (100% - healthy%) of (liability - healthy% of lease)'
        let multiplier = Rational::new(
            Percent::HUNDRED,
            Percent::HUNDRED - self.liability.healthy_percent(),
        );
        let extra_liability = liability_lpn - self.liability.healthy_percent().of(lease_lpn);
        let liquidation_amount =
            <Rational<Percent> as Fraction<Units>>::of(&multiplier, extra_liability);
        let liquidation_amount = lease_lpn.min(liquidation_amount);
        // TODO perform actual liquidation

        let info = LeaseInfo {
            customer,
            ltv: self.liability.max_percent(),
            lease_asset,
        };

        if liquidation_amount == lease_lpn {
            Status::FullLiquidation(info)
        } else {
            Status::PartialLiquidation {
                _info: info,
                _healthy_ltv: self.liability.healthy_percent(),
                liquidation_amount,
            }
        }
    }

    #[inline]
    fn reschedule<A>(
        &mut self,
        lease: A,
        lease_amount: Coin<Lpn>,
        now: &Timestamp,
        liquidation_status: &Status<Lpn>,
    ) -> ContractResult<()>
    where
        A: Into<Addr>,
    {
        self.reschedule_time_alarm(now, liquidation_status)?;

        self.reschedule_price_alarm(lease, lease_amount, now, liquidation_status)
    }

    fn reschedule_price_alarm<A>(
        &mut self,
        lease: A,
        mut lease_amount: Coin<Lpn>,
        now: &Timestamp,
        liquidation_status: &Status<Lpn>,
    ) -> ContractResult<()>
    where
        A: Into<Addr>,
    {
        if self.currency != Lpn::SYMBOL {
            if let Status::PartialLiquidation {
                liquidation_amount, ..
            } = liquidation_status
            {
                lease_amount -= *liquidation_amount;
            }

            let lease = lease.into();

            let (below, above) = match liquidation_status {
                Status::None | Status::PartialLiquidation { .. } => {
                    (self.liability.first_liq_warn_percent(), None)
                }
                Status::Warning(_, WarningLevel::First) => (
                    self.liability.second_liq_warn_percent(),
                    Some(self.liability.first_liq_warn_percent()),
                ),
                Status::Warning(_, WarningLevel::Second) => (
                    self.liability.third_liq_warn_percent(),
                    Some(self.liability.second_liq_warn_percent()),
                ),
                Status::Warning(_, WarningLevel::Third) => (
                    self.liability.max_percent(),
                    Some(self.liability.third_liq_warn_percent()),
                ),
                Status::FullLiquidation(_) => unreachable!(),
            };

            let below = self.price_alarm_by_percent(lease.clone(), lease_amount, now, below)?;

            let above = above
                .map(|above| self.price_alarm_by_percent(lease, lease_amount, now, above))
                .transpose()?;

            self.oracle
                .add_alarm(Alarm::new::<PriceDTO>(
                    self.currency.clone(),
                    below.into(),
                    above.map(Into::into),
                ))
                .map_err(Into::into)
        } else {
            Ok(())
        }
    }

    fn reschedule_time_alarm(
        &mut self,
        now: &Timestamp,
        liquidation_status: &Status<Lpn>,
    ) -> ContractResult<()> {
        debug_assert!(!matches!(liquidation_status, Status::FullLiquidation(..)));

        self.time_alarms
            .add_alarm({
                self.loan
                    .grace_period_end()
                    .min(*now + self.liability.recalculation_time())
            })
            .map_err(Into::into)
    }

    fn price_alarm_by_percent<A>(
        &self,
        lease: A,
        lease_amount: Coin<Lpn>,
        now: &Timestamp,
        percent: Percent,
    ) -> ContractResult<Price<Lpn, Lpn>>
    where
        A: Into<Addr>,
    {
        assert!(!lease_amount.is_zero(), "Loan already paid!");

        Ok(total_of(percent.of(lease_amount))
            .is(self.ltv(*now, lease, lease_amount)?.liability_lpn))
    }
}

pub(crate) struct OnAlarmResult<Lpn>
    where
        Lpn: Currency,
{
    pub batch: Batch,
    pub lease_dto: LeaseDTO,
    pub liquidation_status: Status<Lpn>,
}

#[cfg_attr(test, derive(Debug, Eq, PartialEq))]
pub(crate) enum Status<Lpn>
    where
        Lpn: Currency,
{
    None,
    Warning(LeaseInfo, WarningLevel),
    PartialLiquidation {
        _info: LeaseInfo,
        _healthy_ltv: Percent,
        liquidation_amount: Coin<Lpn>,
    },
    FullLiquidation(LeaseInfo),
}

#[cfg_attr(test, derive(Debug, Eq, PartialEq))]
pub(crate) struct LeaseInfo {
    pub customer: Addr,
    pub ltv: Percent,
    pub lease_asset: SymbolOwned,
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

#[derive(Debug, Eq, PartialEq)]
struct LtvResult<Lpn>
    where
        Lpn: Currency,
{
    pub ltv: Percent,
    pub liability_lpn: Coin<Lpn>,
    pub overdue: Coin<Lpn>,
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::{to_binary, Addr, Timestamp, WasmMsg};

    use finance::currency::Currency;
    use finance::duration::Duration;

    use finance::percent::Percent;
    use finance::price::total_of;
    use lpp::msg::LoanResponse;
    use platform::batch::Batch;
    use time_alarms::msg::ExecuteMsg::AddAlarm;

    use crate::lease::liquidation::LtvResult;
    use crate::lease::tests::TestCurrency;
    use crate::lease::{
        tests::{coin, lease_setup, LEASE_START},
        LeaseInfo, Status, WarningLevel,
    };

    #[test]
    fn warnings_none() {
        let _lease_amount = 1000;
        let interest_rate = Percent::from_permille(50);
        // LPP loan
        let loan = LoanResponse {
            principal_due: coin(500),
            interest_due: coin(100),
            annual_interest_rate: interest_rate,
            interest_paid: Timestamp::from_nanos(0),
        };

        let lease = lease_setup(
            Some(loan),
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
        let _lease_amount = 1000;
        let interest_rate = Percent::from_permille(50);
        // LPP loan
        let loan = LoanResponse {
            principal_due: coin(500),
            interest_due: coin(100),
            annual_interest_rate: interest_rate,
            interest_paid: Timestamp::from_nanos(0),
        };

        let lease = lease_setup(
            Some(loan),
            Addr::unchecked(String::new()),
            Addr::unchecked(String::new()),
        );

        assert_eq!(
            lease.handle_warnings(lease.liability.first_liq_warn_percent()),
            Status::Warning(
                LeaseInfo {
                    customer: lease.customer.clone(),
                    ltv: lease.liability.first_liq_warn_percent(),
                    lease_asset: TestCurrency::SYMBOL.into(),
                },
                WarningLevel::First,
            )
        );
    }

    #[test]
    fn warnings_second() {
        let _lease_amount = 1000;
        let interest_rate = Percent::from_permille(50);
        // LPP loan
        let loan = LoanResponse {
            principal_due: coin(500),
            interest_due: coin(100),
            annual_interest_rate: interest_rate,
            interest_paid: Timestamp::from_nanos(0),
        };

        let lease = lease_setup(
            Some(loan),
            Addr::unchecked(String::new()),
            Addr::unchecked(String::new()),
        );

        assert_eq!(
            lease.handle_warnings(lease.liability.second_liq_warn_percent()),
            Status::Warning(
                LeaseInfo {
                    customer: lease.customer.clone(),
                    ltv: lease.liability.second_liq_warn_percent(),
                    lease_asset: TestCurrency::SYMBOL.into(),
                },
                WarningLevel::Second,
            )
        );
    }

    #[test]
    fn warnings_third() {
        let _lease_amount = 1000;
        let interest_rate = Percent::from_permille(50);
        // LPP loan
        let loan = LoanResponse {
            principal_due: coin(500),
            interest_due: coin(100),
            annual_interest_rate: interest_rate,
            interest_paid: Timestamp::from_nanos(0),
        };

        let lease = lease_setup(
            Some(loan),
            Addr::unchecked(String::new()),
            Addr::unchecked(String::new()),
        );

        assert_eq!(
            lease.handle_warnings(lease.liability.third_liq_warn_percent()),
            Status::Warning(
                LeaseInfo {
                    customer: lease.customer.clone(),
                    ltv: lease.liability.third_liq_warn_percent(),
                    lease_asset: TestCurrency::SYMBOL.into(),
                },
                WarningLevel::Third,
            )
        );
    }

    #[test]
    fn liability() {
        let interest_rate = Percent::from_permille(50);
        // LPP loan
        let loan = LoanResponse {
            principal_due: coin(500),
            interest_due: coin(100),
            annual_interest_rate: interest_rate,
            interest_paid: Timestamp::from_nanos(0),
        };

        let lease = lease_setup(
            Some(loan),
            Addr::unchecked(String::new()),
            Addr::unchecked(String::new()),
        );

        assert_eq!(
            lease
                .ltv(LEASE_START, Addr::unchecked(String::new()), coin(1000))
                .unwrap(),
            LtvResult {
                ltv: Percent::from_percent(60),
                liability_lpn: coin(100 + 500),
                overdue: coin(0),
            }
        );
    }

    #[test]
    fn liquidate_partial() {
        let lease_amount = 1000;
        let interest_rate = Percent::from_permille(50);
        // LPP loan
        let loan = LoanResponse {
            principal_due: coin(500),
            interest_due: coin(100),
            annual_interest_rate: interest_rate,
            interest_paid: Timestamp::from_nanos(0),
        };

        let lease = lease_setup(
            Some(loan),
            Addr::unchecked(String::new()),
            Addr::unchecked(String::new()),
        );

        assert_eq!(
            lease.liquidate(
                Addr::unchecked(String::new()),
                String::new(),
                coin(lease_amount),
                coin(800),
            ),
            Status::PartialLiquidation {
                _info: LeaseInfo {
                    customer: Addr::unchecked(String::new()),
                    ltv: lease.liability.max_percent(),
                    lease_asset: "".into(),
                },
                _healthy_ltv: lease.liability.healthy_percent(),
                liquidation_amount: coin(333),
            }
        );
    }

    #[test]
    fn liquidate_full() {
        let lease_amount = 1000;
        let interest_rate = Percent::from_permille(50);
        // LPP loan
        let loan = LoanResponse {
            principal_due: coin(500),
            interest_due: coin(100),
            annual_interest_rate: interest_rate,
            interest_paid: Timestamp::from_nanos(0),
        };

        let lease = lease_setup(
            Some(loan),
            Addr::unchecked(String::new()),
            Addr::unchecked(String::new()),
        );

        assert_eq!(
            lease.liquidate(
                Addr::unchecked(String::new()),
                String::new(),
                coin(lease_amount),
                coin(5000),
            ),
            Status::FullLiquidation(LeaseInfo {
                customer: Addr::unchecked(String::new()),
                ltv: lease.liability.max_percent(),
                lease_asset: "".into(),
            },)
        );
    }

    #[test]
    fn reschedule_time_alarm_recalc() {
        let interest_rate = Percent::from_permille(50);
        // LPP loan
        let loan = LoanResponse {
            principal_due: coin(300),
            interest_due: coin(0),
            annual_interest_rate: interest_rate,
            interest_paid: Timestamp::from_nanos(0),
        };

        let mut lease = lease_setup(
            Some(loan),
            Addr::unchecked(String::new()),
            Addr::unchecked(String::new()),
        );

        lease
            .reschedule_time_alarm(
                &(lease.loan.grace_period_end()
                    - Duration::from_nanos(lease.liability.recalculation_time().nanos() * 2)),
                &Status::Warning(
                    LeaseInfo {
                        customer: Addr::unchecked(String::new()),
                        ltv: Default::default(),
                        lease_asset: "".to_string(),
                    },
                    WarningLevel::Second,
                ),
            )
            .unwrap();

        assert_eq!(lease.time_alarms.batch, {
            let mut batch = Batch::default();

            batch.schedule_execute_no_reply(WasmMsg::Execute {
                contract_addr: String::new(),
                msg: to_binary(&AddAlarm {
                    time: lease.loan.grace_period_end() - lease.liability.recalculation_time(),
                })
                .unwrap(),
                funds: vec![],
            });

            batch
        });
    }

    #[test]
    fn reschedule_time_alarm_liquidation() {
        let interest_rate = Percent::from_permille(50);
        // LPP loan
        let loan = LoanResponse {
            principal_due: coin(300),
            interest_due: coin(0),
            annual_interest_rate: interest_rate,
            interest_paid: Timestamp::from_nanos(0),
        };

        let mut lease = lease_setup(
            Some(loan),
            Addr::unchecked(String::new()),
            Addr::unchecked(String::new()),
        );

        lease
            .reschedule_time_alarm(
                &(lease.loan.grace_period_end() - lease.liability.recalculation_time()
                    + Duration::from_nanos(1)),
                &Status::Warning(
                    LeaseInfo {
                        customer: Addr::unchecked(String::new()),
                        ltv: Default::default(),
                        lease_asset: "".to_string(),
                    },
                    WarningLevel::Second,
                ),
            )
            .unwrap();

        assert_eq!(lease.time_alarms.batch, {
            let mut batch = Batch::default();

            batch.schedule_execute_no_reply(WasmMsg::Execute {
                contract_addr: String::new(),
                msg: to_binary(&AddAlarm {
                    time: lease.loan.grace_period_end(),
                })
                .unwrap(),
                funds: vec![],
            });

            batch
        });
    }

    #[test]
    #[ignore = "No support for same currency prices. Without Price's debug assertion, runs successfully."]
    fn price_alarm_by_percent() {
        let lease_amount = 1000;
        let interest_rate = Percent::from_permille(50);
        // LPP loan
        let loan = LoanResponse {
            principal_due: coin(300),
            interest_due: coin(0),
            annual_interest_rate: interest_rate,
            interest_paid: Timestamp::from_nanos(0),
        };

        let lease = lease_setup(
            Some(loan),
            Addr::unchecked(String::new()),
            Addr::unchecked(String::new()),
        );

        assert_eq!(
            lease
                .price_alarm_by_percent(
                    Addr::unchecked(String::new()),
                    coin(lease_amount),
                    &LEASE_START,
                    Percent::from_percent(50),
                )
                .unwrap(),
            total_of(coin(5)).is(coin(3))
        );
    }
}
