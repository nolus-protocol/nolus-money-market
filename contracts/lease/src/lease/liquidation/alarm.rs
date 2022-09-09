use cosmwasm_std::{Addr, Timestamp};
use serde::Serialize;

use finance::{
    coin::Coin,
    currency::Currency,
    fraction::Fraction,
    percent::Percent,
    price::{total, total_of, Price, PriceDTO},
};
use lpp::stub::Lpp as LppTrait;
use market_price_oracle::stub::Oracle as OracleTrait;
use marketprice::alarms::Alarm;
use platform::bank::BankAccountView;
use time_alarms::stub::TimeAlarms as TimeAlarmsTrait;

use crate::{
    error::ContractResult,
    lease::{Lease, OnAlarmResult, Status, WarningLevel},
};

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
    ) -> ContractResult<OnAlarmResult<Lpn>>
    where
        B: BankAccountView,
    {
        let lease_amount = account.balance::<Lpn>()?;

        let price_to_lpn = self.price_of_lease_currency()?;

        let status = self.act_on_liability(now, lease.clone(), lease_amount, price_to_lpn)?;

        if !matches!(status, Status::FullLiquidation(_)) {
            self.reschedule(lease, lease_amount, &now, &status, price_to_lpn)?;
        }

        Ok(self.into_on_alarm_result(status))
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

        let price_to_lpn = self.price_of_lease_currency()?;

        let lease_lpn = total(lease_amount, price_to_lpn);

        let status = if self.loan.grace_period_end() <= now {
            self.liquidate_on_interest_overdue(now, lease.clone(), lease_amount, price_to_lpn)?
        } else {
            self.handle_warnings(
                self.loan
                    .liability_status(now, lease.clone(), lease_lpn)?
                    .ltv,
            )
        };

        if !matches!(status, Status::FullLiquidation(_)) {
            self.reschedule(lease, lease_amount, &now, &status, price_to_lpn)?;
        }

        Ok(self.into_on_alarm_result(status))
    }

    #[inline]
    pub(in crate::lease) fn initial_alarm_schedule<A>(
        &mut self,
        lease: A,
        lease_amount: Coin<Lpn>,
        now: &Timestamp,
    ) -> ContractResult<()>
    where
        A: Into<Addr>,
    {
        self.reschedule(
            lease,
            lease_amount,
            now,
            &Status::None,
            self.price_of_lease_currency()?,
        )
    }

    #[inline]
    pub(in crate::lease) fn reschedule_on_repay<A>(
        &mut self,
        lease: A,
        lease_amount: Coin<Lpn>,
        now: &Timestamp,
    ) -> ContractResult<()>
    where
        A: Into<Addr> + Clone,
    {
        let price_to_lpn = self.price_of_lease_currency()?;

        let lease_lpn = total(lease_amount, price_to_lpn);

        self.reschedule(
            lease.clone(),
            lease_amount,
            now,
            &self.handle_warnings(self.loan.liability_status(*now, lease, lease_lpn)?.ltv),
            price_to_lpn,
        )
    }

    fn into_on_alarm_result(self, liquidation_status: Status<Lpn>) -> OnAlarmResult<Lpn> {
        let (lease_dto, batch) = self.into_dto();

        OnAlarmResult {
            batch,
            lease_dto,
            liquidation_status,
        }
    }

    #[inline]
    fn reschedule<A>(
        &mut self,
        lease: A,
        lease_amount: Coin<Lpn>,
        now: &Timestamp,
        liquidation_status: &Status<Lpn>,
        price_to_lpn: Price<Lpn, Lpn>,
    ) -> ContractResult<()>
    where
        A: Into<Addr>,
    {
        self.reschedule_time_alarm(now, liquidation_status)?;

        self.reschedule_price_alarm(lease, lease_amount, now, liquidation_status, price_to_lpn)
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

    fn reschedule_price_alarm<A>(
        &mut self,
        lease: A,
        mut lease_amount: Coin<Lpn>,
        now: &Timestamp,
        liquidation_status: &Status<Lpn>,
        price_to_lpn: Price<Lpn, Lpn>,
    ) -> ContractResult<()>
    where
        A: Into<Addr>,
    {
        if self.currency == Lpn::SYMBOL {
            return Ok(());
        }

        if let Status::PartialLiquidation {
            liquidation_amount, ..
        } = liquidation_status
        {
            lease_amount -= *liquidation_amount;
        }

        let lease_lpn = total(lease_amount, price_to_lpn);

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

        let total_liability = self
            .loan
            .liability_status(*now + self.liability.recalculation_time(), lease, lease_lpn)?
            .total_lpn;

        let below = self.price_alarm_by_percent(lease_amount, total_liability, below)?;

        let above = above
            .map(|above| self.price_alarm_by_percent(lease_amount, total_liability, above))
            .transpose()?;

        self.oracle
            .add_alarm(Alarm::new::<PriceDTO>(
                self.currency.clone(),
                below.into(),
                above.map(Into::into),
            ))
            .map_err(Into::into)
    }

    fn price_alarm_by_percent(
        &self,
        lease_amount: Coin<Lpn>,
        liability: Coin<Lpn>,
        percent: Percent,
    ) -> ContractResult<Price<Lpn, Lpn>> {
        assert!(!lease_amount.is_zero(), "Loan already paid!");

        Ok(total_of(percent.of(lease_amount)).is(liability))
    }
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::{to_binary, Addr, Timestamp, WasmMsg};

    use finance::{duration::Duration, percent::Percent, price::total_of};
    use lpp::msg::LoanResponse;
    use platform::batch::Batch;
    use time_alarms::msg::ExecuteMsg::AddAlarm;

    use crate::{
        lease::tests::{coin, lease_setup},
        lease::{LeaseInfo, Status, WarningLevel},
    };

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
                    - lease.liability.recalculation_time()
                    - lease.liability.recalculation_time()),
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
                .price_alarm_by_percent(coin(lease_amount), coin(500), Percent::from_percent(50))
                .unwrap(),
            total_of(coin(5)).is(coin(3))
        );
    }
}
