use currency::{Currency, CurrencyDef, MemberOf};
use finance::{duration::Duration, liability::Zone, price::base::BasePrice};
use oracle::{
    api::alarms::Alarm,
    stub::{AsAlarms, PriceAlarms as PriceAlarmsTrait},
};
use platform::batch::Batch;
use sdk::cosmwasm_std::Timestamp;
use timealarms::stub::TimeAlarmsRef;

use crate::{
    error::ContractResult,
    finance::{LpnCoin, LpnCurrency, OracleRef},
    lease::Lease,
};

use super::{range::SteadyPriceRange, LeaseAssetCurrencies};

impl<Asset, Lpp, Oracle> Lease<Asset, Lpp, Oracle>
where
    Asset: Currency,
{
    pub(super) fn steadiness(
        &self,
        now: &Timestamp,
        recheck_in: Duration,
        liquidation_zone: &Zone,
        total_due: LpnCoin,
    ) -> Steadiness<Asset> {
        Steadiness {
            by: now + recheck_in,
            within: self.price_range(liquidation_zone, total_due),
        }
    }

    fn price_range(&self, liquidation_zone: &Zone, total_due: LpnCoin) -> SteadyPriceRange<Asset> {
        debug_assert!(!currency::equal::<LpnCurrency, Asset>());
        debug_assert!(!total_due.is_zero());

        let above_incl = self.position.price_at(liquidation_zone.high(), total_due);

        let below_excl = liquidation_zone
            .low()
            .map(|low| self.position.price_at(low, total_due));
        SteadyPriceRange::new(above_incl, below_excl)
    }
}

#[cfg_attr(test, derive(Debug, PartialEq, Eq))]
pub(crate) struct Steadiness<Asset>
where
    Asset: 'static,
{
    by: Timestamp,
    within: SteadyPriceRange<Asset>,
}

impl<Asset> Steadiness<Asset>
where
    Asset: CurrencyDef,
    Asset::Group: MemberOf<LeaseAssetCurrencies>,
{
    pub fn try_into_alarms(
        self,
        time_alarms: &TimeAlarmsRef,
        price_alarms: &OracleRef,
    ) -> ContractResult<Batch> {
        time_alarms
            .setup_alarm(self.by)
            .map_err(Into::into)
            .and_then(|schedule_time_alarm| {
                let mut price_alarms = price_alarms.as_alarms::<LeaseAssetCurrencies>();
                price_alarms
                    .add_alarm(Alarm::new(
                        // NOTE: we miss alarms in the exact case when the price == SteadyPriceRange::above_excl
                        // This is due to the discrepancy of the openness of liability LTV ranges and alarms.
                        // While the former are closed at the start and open at the end, the latter are in reverse.
                        // The best solution is to turn 'below' into 'below_or_equal' and 'above_or_equal' into 'above'.
                        self.within.above_excl().into(),
                        self.within
                            .may_below_incl()
                            .map(Into::<BasePrice<LeaseAssetCurrencies, _, _>>::into),
                    ))
                    .map(|_| schedule_time_alarm.merge(price_alarms.into()))
                    .map_err(Into::into)
            })
    }
}

#[cfg(test)]
mod tests {
    use finance::{
        coin::Coin,
        duration::Duration,
        fraction::Fraction,
        liability::Zone,
        percent::Percent,
        price::{self, total_of, Price},
    };
    use lpp::msg::LoanResponse;

    use crate::{
        lease::{
            self,
            range::SteadyPriceRange,
            steady::Steadiness,
            tests::{
                loan, open_lease, FIRST_LIQ_WARN, LEASE_START, RECHECK_TIME, SECOND_LIQ_WARN,
                THIRD_LIQ_WARN,
            },
        },
        position::DueTrait,
    };

    #[test]
    fn initial_alarms() {
        let asset = Coin::from(10);
        let lease = lease::tests::open_lease(asset, loan());
        let now = LEASE_START;
        let recheck_time = now + RECHECK_TIME;
        let liability_alarm_on = FIRST_LIQ_WARN;
        let due = {
            let lease = &lease;
            lease.loan.state(&now)
        };
        let steadiness = lease.steadiness(
            &LEASE_START,
            RECHECK_TIME,
            &Zone::no_warnings(liability_alarm_on),
            due.total_due(),
        );

        let above_excl_price = total_of(liability_alarm_on.of(asset)).is(due.total_due());
        assert_eq!(
            Steadiness {
                by: recheck_time,
                within: SteadyPriceRange::new(above_excl_price, None)
            },
            steadiness
        );
    }

    #[test]
    fn third_zone_alarms() {
        let principal = 300.into();
        let interest_rate = Percent::from_permille(145);

        let loan = LoanResponse {
            principal_due: principal,
            annual_interest_rate: interest_rate,
            interest_paid: LEASE_START,
        };

        let lease_amount = 1000.into();
        let lease = open_lease(lease_amount, loan);

        let reschedule_at = LEASE_START + Duration::from_days(50);
        let recalc_at = reschedule_at + RECHECK_TIME;

        let zone = Zone::second(SECOND_LIQ_WARN, THIRD_LIQ_WARN);
        let total_due = price::total(
            (SECOND_LIQ_WARN + Percent::from_percent(1)).of(lease_amount),
            Price::identity(),
        );
        let steadiness = lease.steadiness(&reschedule_at, RECHECK_TIME, &zone, total_due);

        let exp_below = total_of(zone.high().ltv().of(lease_amount)).is(total_due);
        let exp_above = total_of(zone.low().unwrap().ltv().of(lease_amount)).is(total_due);

        assert_eq!(
            Steadiness {
                by: recalc_at,
                within: SteadyPriceRange::new(exp_below, Some(exp_above))
            },
            steadiness
        );
    }

    mod into_alarms {
        use cosmwasm_std::Timestamp;
        use currencies::{LeaseGroup, Lpns};
        use finance::{coin::Coin, price};
        use oracle::api::alarms::{ExecuteMsg as PriceAlarmsCmd, Alarm};
        use oracle_platform::OracleRef;
        use platform::batch::Batch;
        use sdk::cosmwasm_std::{self, Addr, WasmMsg};
        use timealarms::{msg::ExecuteMsg as TimeAlarmsCmd, stub::TimeAlarmsRef};

        use crate::lease::{
            range::SteadyPriceRange,
            steady::Steadiness,
            tests::{TestCurrency, TestLpn},
        };

        const TIME_ALARMS_ADDR: &str = "timealarms";
        const ORACLE_ADDR: &str = "oracle";

        #[test]
        fn try_into_alarms() {
            let recheck_time = Timestamp::from_seconds(1732016180);
            let above_excl = price::total_of::<TestCurrency>(Coin::from(10)).is(Coin::from(45));

            let s = Steadiness {
                by: recheck_time,
                within: SteadyPriceRange::new(above_excl, None),
            };

            assert_eq!(s.try_into_alarms(&timealarms(), &pricealarms()), {
                let mut batch = Batch::default();

                batch.schedule_execute_no_reply(WasmMsg::Execute {
                    contract_addr: TIME_ALARMS_ADDR.into(),
                    msg: cosmwasm_std::to_json_binary(&TimeAlarmsCmd::AddAlarm {
                        time: recheck_time,
                    })
                    .unwrap(),
                    funds: vec![],
                });

                batch.schedule_execute_no_reply(WasmMsg::Execute {
                    contract_addr: ORACLE_ADDR.into(),
                    msg: cosmwasm_std::to_json_binary(&PriceAlarmsCmd::AddPriceAlarm::<
                        LeaseGroup,
                        TestLpn,
                        Lpns,
                    > {
                        alarm: Alarm::new(above_excl, None),
                    })
                    .unwrap(),
                    funds: vec![],
                });

                Ok(batch)
            });
        }

        fn timealarms() -> TimeAlarmsRef {
            TimeAlarmsRef::unchecked(TIME_ALARMS_ADDR)
        }

        fn pricealarms() -> OracleRef<TestLpn, Lpns> {
            OracleRef::unchecked(Addr::unchecked(ORACLE_ADDR))
        }
    }
}
