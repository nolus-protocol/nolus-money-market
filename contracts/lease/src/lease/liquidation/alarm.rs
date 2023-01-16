use serde::Serialize;

use finance::{
    coin::Coin,
    currency::{self, Currency},
    fraction::Fraction,
    percent::Percent,
    price::{total, total_of, Price},
};
use lpp::stub::lender::LppLender as LppLenderTrait;
use marketprice::SpotPrice;
use oracle::{alarms::Alarm, stub::Oracle as OracleTrait};
use profit::stub::Profit as ProfitTrait;
use sdk::cosmwasm_std::Timestamp;
use timealarms::stub::TimeAlarms as TimeAlarmsTrait;

use crate::{
    error::ContractResult,
    lease::{IntoDTOResult, Lease, LiquidationInfo, OnAlarmResult, Status, WarningLevel},
    loan::LiabilityStatus,
};

impl<'r, Lpn, Asset, Lpp, Profit, TimeAlarms, Oracle>
    Lease<'r, Lpn, Asset, Lpp, Profit, TimeAlarms, Oracle>
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
        self.reschedule(self.lease_amount_lpn()?, now, &Status::None)
    }

    pub(crate) fn on_price_alarm(
        self,
        now: Timestamp,
    ) -> ContractResult<OnAlarmResult<Lpn, Asset>> {
        self.on_alarm(Self::act_on_liability, now)
    }

    pub(crate) fn on_time_alarm(self, now: Timestamp) -> ContractResult<OnAlarmResult<Lpn, Asset>> {
        self.on_alarm(Self::act_on_overdue, now)
    }

    #[inline]
    pub(in crate::lease) fn reschedule_on_repay(&mut self, now: &Timestamp) -> ContractResult<()> {
        let lease_lpn = self.lease_amount_lpn()?;

        self.reschedule(
            lease_lpn,
            now,
            &self.handle_warnings(
                self.loan
                    .liability_status(*now, self.lease_addr.clone(), lease_lpn)?
                    .ltv,
            ),
        )
    }

    fn on_alarm<F>(
        mut self,
        handler: F,
        now: Timestamp,
    ) -> ContractResult<OnAlarmResult<Lpn, Asset>>
    where
        F: FnOnce(
            &mut Self,
            Coin<Lpn>,
            Timestamp,
            Percent,
            Coin<Lpn>,
        ) -> ContractResult<Status<Lpn, Asset>>,
    {
        let price_to_lpn = self.price_of_lease_currency()?;

        let lease_lpn = total(self.amount, price_to_lpn);

        let LiabilityStatus {
            ltv,
            total_lpn: liability_lpn,
            ..
        } = self
            .loan
            .liability_status(now, self.lease_addr.clone(), lease_lpn)?;

        let status = handler(&mut self, lease_lpn, now, ltv, liability_lpn)?;

        if let Status::PartialLiquidation {
            liquidation_info: LiquidationInfo { receipt, .. },
            ..
        } = &status
        {
            self.amount -= total(receipt.total(), price_to_lpn.inv());
        }

        if !matches!(status, Status::FullLiquidation { .. }) {
            self.reschedule(lease_lpn, &now, &status)?;
        }

        Ok(self.into_on_alarm_result(status))
    }

    fn into_on_alarm_result(
        self,
        liquidation_status: Status<Lpn, Asset>,
    ) -> OnAlarmResult<Lpn, Asset> {
        let IntoDTOResult {
            lease: lease_dto,
            batch,
        } = self.into_dto();

        OnAlarmResult {
            batch,
            lease_dto,
            liquidation_status,
        }
    }

    #[inline]
    fn reschedule(
        &mut self,
        lease_lpn: Coin<Lpn>,
        now: &Timestamp,
        liquidation_status: &Status<Lpn, Asset>,
    ) -> ContractResult<()> {
        self.reschedule_time_alarm(now, liquidation_status)?;

        self.reschedule_price_alarm(lease_lpn, now, liquidation_status)
    }

    fn reschedule_time_alarm(
        &mut self,
        now: &Timestamp,
        liquidation_status: &Status<Lpn, Asset>,
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
        lease_lpn: Coin<Lpn>,
        now: &Timestamp,
        liquidation_status: &Status<Lpn, Asset>,
    ) -> ContractResult<()> {
        if currency::equal::<Asset, Lpn>() {
            return Ok(());
        }

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
            Status::FullLiquidation { .. } => unreachable!(),
        };

        let total_liability = self
            .loan
            .liability_status(
                *now + self.liability.recalculation_time(),
                self.lease_addr.clone(),
                lease_lpn,
            )?
            .total_lpn;

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
    use finance::{duration::Duration, percent::Percent, price::total_of};
    use lpp::msg::LoanResponse;
    use platform::batch::Batch;
    use sdk::cosmwasm_std::{to_binary, Addr, Timestamp, WasmMsg};
    use timealarms::msg::ExecuteMsg::AddAlarm;

    use crate::lease::{
        self,
        tests::{
            coin, open_lease, LppLenderLocalStubUnreachable, OracleLocalStub,
            ProfitLocalStubUnreachable, TimeAlarmsLocalStub, LEASE_START,
        },
        LeaseInfo, Status, WarningLevel,
    };

    #[test]
    fn initial_alarm_schedule() {
        let lease_addr = Addr::unchecked("lease");
        let lease = lease::tests::create_lease(
            &lease_addr,
            10.into(),
            LppLenderLocalStubUnreachable {},
            TimeAlarmsLocalStub::from(Addr::unchecked(String::new())),
            OracleLocalStub::from(Addr::unchecked(String::new())),
            ProfitLocalStubUnreachable,
        );
        assert_eq!(lease.alarms.batch, {
            let mut batch = Batch::default();

            batch.schedule_execute_no_reply(WasmMsg::Execute {
                contract_addr: String::new(),
                msg: to_binary(&AddAlarm {
                    time: LEASE_START + lease.liability.recalculation_time(),
                })
                .unwrap(),
                funds: vec![],
            });

            batch
        });
    }

    #[test]
    fn reschedule_time_alarm_recalc() {
        let loan = LoanResponse {
            principal_due: coin(300),
            interest_due: coin(0),
            annual_interest_rate: Percent::from_permille(50),
            interest_paid: Timestamp::from_nanos(0),
        };

        let lease_addr = Addr::unchecked("lease");
        let mut lease = open_lease(
            &lease_addr,
            20.into(),
            Some(loan),
            Addr::unchecked(String::new()),
            Addr::unchecked(String::new()),
            Addr::unchecked(String::new()),
        );

        lease
            .reschedule_time_alarm(
                &(lease.loan.grace_period_end()
                    - lease.liability.recalculation_time()
                    - lease.liability.recalculation_time()),
                &Status::Warning(
                    LeaseInfo::new(
                        Addr::unchecked(String::new()),
                        lease_addr.clone(),
                        Default::default(),
                    ),
                    WarningLevel::Second,
                ),
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
        let interest_rate = Percent::from_permille(50);
        // LPP loan
        let loan = LoanResponse {
            principal_due: coin(300),
            interest_due: coin(0),
            annual_interest_rate: interest_rate,
            interest_paid: Timestamp::from_nanos(0),
        };

        let lease_addr = Addr::unchecked("lease");
        let mut lease = open_lease(
            &lease_addr,
            300.into(),
            Some(loan),
            Addr::unchecked(String::new()),
            Addr::unchecked(String::new()),
            Addr::unchecked(String::new()),
        );

        lease
            .reschedule_time_alarm(
                &(lease.loan.grace_period_end() - lease.liability.recalculation_time()
                    + Duration::from_nanos(1)),
                &Status::Warning(
                    LeaseInfo::new(
                        Addr::unchecked(String::new()),
                        lease_addr.clone(),
                        Default::default(),
                    ),
                    WarningLevel::Second,
                ),
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
    #[ignore = "No support for same currency prices. Without Price's debug assertion, runs successfully."]
    fn price_alarm_by_percent() {
        let interest_rate = Percent::from_permille(50);
        // LPP loan
        let loan = LoanResponse {
            principal_due: coin(300),
            interest_due: coin(0),
            annual_interest_rate: interest_rate,
            interest_paid: Timestamp::from_nanos(0),
        };

        let lease_addr = Addr::unchecked("lease");
        let lease = open_lease(
            &lease_addr,
            300.into(),
            Some(loan),
            Addr::unchecked(String::new()),
            Addr::unchecked(String::new()),
            Addr::unchecked(String::new()),
        );

        assert_eq!(
            lease
                .price_alarm_by_percent(coin(500), Percent::from_percent(50))
                .unwrap(),
            total_of(coin(5)).is(coin(3))
        );
    }
}
