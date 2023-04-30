use serde::Serialize;

use finance::{
    coin::Coin,
    currency::{self, Currency},
    fraction::Fraction,
    liability::{Level, Zone},
    price::{total_of, Price},
};
use lpp::stub::lender::LppLender as LppLenderTrait;
use marketprice::SpotPrice;
use oracle::{alarms::Alarm, stub::Oracle as OracleTrait};
use profit::stub::Profit as ProfitTrait;
use sdk::cosmwasm_std::Timestamp;
use timealarms::stub::TimeAlarms as TimeAlarmsTrait;

use crate::{error::ContractResult, lease::Lease};

impl<Lpn, Asset, Lpp, Profit, TimeAlarms, Oracle> Lease<Lpn, Asset, Lpp, Profit, TimeAlarms, Oracle>
where
    Lpn: Currency + Serialize,
    Lpp: LppLenderTrait<Lpn>,
    TimeAlarms: TimeAlarmsTrait,
    Oracle: OracleTrait<Lpn>,
    Profit: ProfitTrait,
    Asset: Currency + Serialize,
{
    //TODO keep loan state updated on payments and liquidations to have the liquidation status accurate
    //do it at the LppStub
    pub(crate) fn reschedule(
        &mut self,
        now: &Timestamp,
        liquidation_zone: &Zone,
    ) -> ContractResult<()> {
        self.reschedule_time_alarm(now)?;

        self.reschedule_price_alarm(now, liquidation_zone)
    }

    fn reschedule_time_alarm(&mut self, now: &Timestamp) -> ContractResult<()> {
        self.alarms
            .add_alarm(
                self.loan
                    .grace_period_end()
                    .min(*now + self.liability.recalculation_time()),
            )
            .map_err(Into::into)
    }

    fn reschedule_price_alarm(
        &mut self,
        now: &Timestamp,
        liquidation_zone: &Zone,
    ) -> ContractResult<()> {
        debug_assert!(!currency::equal::<Lpn, Asset>());

        let total_liability = self
            .loan
            .liability_status(
                *now + self.liability.recalculation_time(),
                self.addr.clone(),
            )?
            .total;

        let below = self.price_alarm_at_level(total_liability, liquidation_zone.high())?;

        let above_or_equal = liquidation_zone
            .low()
            .map(|low| self.price_alarm_at_level(total_liability, low))
            .transpose()?;

        self.oracle
            .add_alarm(Alarm::new(
                below.into(),
                above_or_equal.map(Into::<SpotPrice>::into),
            ))
            .map_err(Into::into)
    }

    fn price_alarm_at_level(
        &self,
        liability: Coin<Lpn>,
        alarm_at: Level,
    ) -> ContractResult<Price<Asset, Lpn>> {
        debug_assert!(!self.amount.is_zero(), "Loan already paid!");

        Ok(total_of(alarm_at.ltv().of(self.amount)).is(liability))
    }
}

#[cfg(test)]
mod tests {
    use finance::liability::Zone;
    use finance::percent::Percent;
    use finance::{coin::Coin, duration::Duration, fraction::Fraction, price::total_of};
    use lpp::msg::LoanResponse;
    use marketprice::SpotPrice;
    use oracle::{alarms::Alarm, msg::ExecuteMsg::AddPriceAlarm};
    use platform::batch::Batch;
    use sdk::cosmwasm_std::{to_binary, Addr, WasmMsg};
    use timealarms::msg::ExecuteMsg::AddAlarm;

    use crate::lease::{
        self,
        tests::{loan, open_lease, LEASE_START},
    };

    #[test]
    fn initial_alarm_schedule() {
        let asset = Coin::from(10);
        let lease_addr = Addr::unchecked("lease");
        let timealarms_addr = Addr::unchecked("timealarms");
        let oracle_addr = Addr::unchecked("oracle");
        let mut lease = lease::tests::open_lease(
            lease_addr,
            asset,
            Some(loan()),
            timealarms_addr.clone(),
            oracle_addr.clone(),
            Addr::unchecked(String::new()),
        );
        let recalc_time = LEASE_START + lease.liability.recalculation_time();
        let liability_alarm_on = lease.liability.first_liq_warn();
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
        lease
            .reschedule(
                &LEASE_START,
                &Zone::no_warnings(lease.liability.first_liq_warn()),
            )
            .unwrap();

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
            )
            .unwrap();
        let recalc_time = lease.loan.grace_period_end() - lease.liability.recalculation_time();

        assert_eq!(lease.into_dto().batch, {
            let mut batch = Batch::default();

            batch.schedule_execute_no_reply(WasmMsg::Execute {
                contract_addr: String::new(),
                msg: to_binary(&AddAlarm { time: recalc_time }).unwrap(),
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
            )
            .unwrap();
        let recalc_time = lease.loan.grace_period_end();

        assert_eq!(lease.into_dto().batch, {
            let mut batch = Batch::default();

            batch.schedule_execute_no_reply(WasmMsg::Execute {
                contract_addr: String::new(),
                msg: to_binary(&AddAlarm { time: recalc_time }).unwrap(),
                funds: vec![],
            });

            batch
        });
    }

    #[test]
    fn price_alarm_by_percent() {
        let principal = 300.into();
        let interest_rate = Percent::from_permille(145);

        let loan = LoanResponse {
            principal_due: principal,
            annual_interest_rate: interest_rate,
            interest_paid: LEASE_START,
        };

        let lease_addr = Addr::unchecked("lease");
        let lease_amount = 1000.into();
        let mut lease = open_lease(
            lease_addr,
            lease_amount,
            Some(loan),
            Addr::unchecked(String::new()),
            Addr::unchecked(String::new()),
            Addr::unchecked(String::new()),
        );

        let reschedule_at = LEASE_START + Duration::from_days(50);
        let projected_liability = {
            let l = lease
                .loan
                .state(
                    reschedule_at + lease.liability.recalculation_time(),
                    lease.addr.clone(),
                )
                .unwrap()
                .unwrap();
            l.principal_due
                + l.previous_interest_due
                + l.previous_margin_interest_due
                + l.current_interest_due
                + l.current_margin_interest_due
        };
        dbg!(projected_liability);

        let zone = Zone::second(
            lease.liability.second_liq_warn(),
            lease.liability.third_liq_warn(),
        );
        lease.reschedule_price_alarm(&reschedule_at, &zone).unwrap();
        let exp_below: SpotPrice = total_of(zone.high().ltv().of(lease_amount))
            .is(projected_liability)
            .into();
        let exp_above: SpotPrice = total_of(zone.low().unwrap().ltv().of(lease_amount))
            .is(projected_liability)
            .into();

        assert_eq!(lease.into_dto().batch, {
            let mut batch = Batch::default();

            batch.schedule_execute_no_reply(WasmMsg::Execute {
                contract_addr: String::new(),
                msg: to_binary(&AddPriceAlarm {
                    alarm: Alarm::new(exp_below, Some(exp_above)),
                })
                .unwrap(),
                funds: vec![],
            });

            batch
        });
    }
}
