use serde::Serialize;

use finance::{
    coin::Coin,
    currency::{self, Currency},
    fraction::Fraction,
    percent::Percent,
    price::{total, total_of, Price},
    zero::Zero,
};
use lpp::stub::lender::LppLender as LppLenderTrait;
use marketprice::SpotPrice;
use oracle::{alarms::Alarm, stub::Oracle as OracleTrait};
use profit::stub::Profit as ProfitTrait;
use sdk::cosmwasm_std::Timestamp;
use timealarms::stub::TimeAlarms as TimeAlarmsTrait;

use crate::{
    error::ContractResult,
    lease::{Lease, Status, WarningLevel},
    loan::LiabilityStatus,
};

impl<Lpn, Asset, Lpp, Profit, TimeAlarms, Oracle> Lease<Lpn, Asset, Lpp, Profit, TimeAlarms, Oracle>
where
    Lpn: Currency + Serialize,
    Lpp: LppLenderTrait<Lpn>,
    TimeAlarms: TimeAlarmsTrait,
    Oracle: OracleTrait<Lpn>,
    Profit: ProfitTrait,
    Asset: Currency + Serialize,
{
    #[inline]
    pub(in crate::lease) fn initial_alarm_schedule(
        &mut self,
        now: &Timestamp,
    ) -> ContractResult<()> {
        self.reschedule(now, &Status::None)
    }

    #[inline]
    pub(in crate::lease) fn reschedule_on_repay(&mut self, now: &Timestamp) -> ContractResult<()> {
        self.reschedule(now, &self.liquidation_status(*now)?)
    }

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

        Ok(super::check_liability(
            &self.liability,
            self.amount,
            total(total_due, price_to_asset),
            total(overdue, price_to_asset),
        ))
    }

    #[inline]
    fn reschedule(
        &mut self,
        now: &Timestamp,
        liquidation_status: &Status<Asset>,
    ) -> ContractResult<()> {
        self.reschedule_time_alarm(now, liquidation_status)?;

        self.reschedule_price_alarm(now, liquidation_status)
    }

    fn reschedule_time_alarm(
        &mut self,
        now: &Timestamp,
        liquidation_status: &Status<Asset>,
    ) -> ContractResult<()> {
        debug_assert!(!matches!(
            liquidation_status,
            Status::FullLiquidation { .. }
        ));

        self.alarms
            .add_alarm({
                self.loan
                    .grace_period_end()
                    .min(*now + self.liability.recalculation_time())
            })
            .map_err(Into::into)
    }

    fn reschedule_price_alarm(
        &mut self,
        now: &Timestamp,
        liquidation_status: &Status<Asset>,
    ) -> ContractResult<()> {
        debug_assert!(!currency::equal::<Lpn, Asset>());

        let (below, above) = match liquidation_status {
            Status::None | Status::PartialLiquidation { .. } => {
                (self.liability.first_liq_warn_percent(), None)
            }
            Status::Warning {
                ltv: _,
                level: WarningLevel::First,
            } => (
                self.liability.second_liq_warn_percent(),
                Some(self.liability.first_liq_warn_percent()),
            ),
            Status::Warning {
                ltv: _,
                level: WarningLevel::Second,
            } => (
                self.liability.third_liq_warn_percent(),
                Some(self.liability.second_liq_warn_percent()),
            ),
            Status::Warning {
                ltv: _,
                level: WarningLevel::Third,
            } => (
                self.liability.max_percent(),
                Some(self.liability.third_liq_warn_percent()),
            ),
            Status::FullLiquidation { .. } => unreachable!(),
        };

        let total_liability = self
            .loan
            .liability_status(
                *now + self.liability.recalculation_time(),
                self.addr.clone(),
            )?
            .total;

        let below = self.price_alarm_by_percent(total_liability, below)?;

        let above = above
            .map(|above| self.price_alarm_by_percent(total_liability, above))
            .transpose()?;

        self.oracle
            .add_alarm(Alarm::new(below.into(), above.map(Into::<SpotPrice>::into)))
            .map_err(Into::into)
    }

    fn price_alarm_by_percent(
        &self,
        liability: Coin<Lpn>,
        percent: Percent,
    ) -> ContractResult<Price<Asset, Lpn>> {
        debug_assert!(!self.amount.is_zero(), "Loan already paid!");

        Ok(total_of(percent.of(self.amount)).is(liability))
    }
}

#[cfg(test)]
mod tests {
    use currency::{lease::Cro, lpn::Usdc};
    use finance::percent::Percent;
    use finance::{coin::Coin, duration::Duration, fraction::Fraction, price::total_of};
    use lpp::msg::LoanResponse;
    use marketprice::SpotPrice;
    use oracle::{alarms::Alarm, msg::ExecuteMsg::AddPriceAlarm};
    use platform::batch::Batch;
    use sdk::cosmwasm_std::Timestamp;
    use sdk::cosmwasm_std::{to_binary, Addr, WasmMsg};
    use timealarms::msg::ExecuteMsg::AddAlarm;

    use crate::lease::{
        self,
        tests::{
            loan, open_lease, LppLenderLocalStub, OracleLocalStub, ProfitLocalStubUnreachable,
            TimeAlarmsLocalStub, LEASE_START,
        },
        Status, WarningLevel,
    };

    #[test]
    fn initial_alarm_schedule() {
        type Lpn = Usdc;
        type Asset = Cro;
        let asset = Coin::from(10);
        let lease_addr = Addr::unchecked("lease");
        let timealarms_addr = Addr::unchecked("timealarms");
        let oracle_addr = Addr::unchecked("oracle");
        let lease = lease::tests::create_lease::<Lpn, Asset, _, _, _, _>(
            lease_addr,
            asset,
            LppLenderLocalStub::from(Some(loan())),
            TimeAlarmsLocalStub::from(timealarms_addr.clone()),
            OracleLocalStub::from(oracle_addr.clone()),
            ProfitLocalStubUnreachable,
        );
        let recalc_time = LEASE_START + lease.liability.recalculation_time();
        let liability_alarm_on = lease.liability.first_liq_warn_percent();
        let projected_liability = {
            let l = lease
                .loan
                .state(recalc_time, lease.addr.clone())
                .unwrap()
                .unwrap();
            l.principal_due
                + l.previous_interest_due
                + l.previous_margin_interest_due
                + l.current_interest_due
                + l.current_margin_interest_due
        };

        assert_eq!(lease.into_dto().batch, {
            let mut batch = Batch::default();

            batch.schedule_execute_no_reply(WasmMsg::Execute {
                contract_addr: timealarms_addr.into(),
                msg: to_binary(&AddAlarm { time: recalc_time }).unwrap(),
                funds: vec![],
            });

            let below_alarm: SpotPrice = total_of(liability_alarm_on.of(asset))
                .is(projected_liability)
                .into();
            batch.schedule_execute_no_reply(WasmMsg::Execute {
                contract_addr: oracle_addr.into(),
                msg: to_binary(&AddPriceAlarm {
                    alarm: Alarm::new(below_alarm, None),
                })
                .unwrap(),
                funds: vec![],
            });

            batch
        });
    }

    #[test]
    fn reschedule_time_alarm_recalc() {
        let lease_addr = Addr::unchecked("lease");
        let mut lease = open_lease(
            lease_addr,
            20.into(),
            Some(loan()),
            Addr::unchecked(String::new()),
            Addr::unchecked(String::new()),
            Addr::unchecked(String::new()),
        );
        lease
            .reschedule_time_alarm(
                &(lease.loan.grace_period_end()
                    - lease.liability.recalculation_time()
                    - lease.liability.recalculation_time()),
                &Status::Warning {
                    ltv: Default::default(),
                    level: WarningLevel::Second,
                },
            )
            .unwrap();
        assert_eq!(lease.alarms.batch, {
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
        let lease_addr = Addr::unchecked("lease");
        let mut lease = open_lease(
            lease_addr,
            300.into(),
            Some(loan()),
            Addr::unchecked(String::new()),
            Addr::unchecked(String::new()),
            Addr::unchecked(String::new()),
        );

        lease
            .reschedule_time_alarm(
                &(lease.loan.grace_period_end() - lease.liability.recalculation_time()
                    + Duration::from_nanos(1)),
                &Status::Warning {
                    ltv: Default::default(),
                    level: WarningLevel::Second,
                },
            )
            .unwrap();

        assert_eq!(lease.alarms.batch, {
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
    fn price_alarm_by_percent() {
        let principal = 300.into();
        let interest_rate = Percent::from_permille(50);
        // LPP loan
        let loan = LoanResponse {
            principal_due: principal,
            annual_interest_rate: interest_rate,
            interest_paid: Timestamp::from_nanos(0),
        };

        let lease_addr = Addr::unchecked("lease");
        let lease_amount = 1000.into();
        let lease = open_lease(
            lease_addr,
            lease_amount,
            Some(loan),
            Addr::unchecked(String::new()),
            Addr::unchecked(String::new()),
            Addr::unchecked(String::new()),
        );
        let alarm_at = Percent::from_percent(80);
        assert_eq!(
            lease.price_alarm_by_percent(principal, alarm_at).unwrap(),
            total_of(alarm_at.of(lease_amount)).is(principal)
        );
    }
}
