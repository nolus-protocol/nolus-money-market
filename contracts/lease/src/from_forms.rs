use cosmwasm_std::{Addr, Api, QuerierWrapper, Timestamp};

use finance::currency::Currency;
use lpp::stub::lender::{LppLender as LppLenderTrait, LppLenderRef};
use market_price_oracle::stub::{Oracle as OracleTrait, OracleRef};
use profit::stub::{Profit as ProfitTrait, ProfitRef};
use serde::Serialize;
use time_alarms::stub::{TimeAlarms as TimeAlarmsTrait, TimeAlarmsRef};

use crate::{
    error::{ContractError, ContractResult},
    lease::{self, IntoDTOResult, Lease, WithLeaseDeps},
    loan::Loan,
    msg::NewLeaseForm,
};

impl NewLeaseForm {
    pub(crate) fn into_lease(
        self,
        addr: &Addr,
        start_at: Timestamp,
        api: &dyn Api,
        querier: &QuerierWrapper,
        lpp: LppLenderRef,
        oracle: OracleRef,
    ) -> ContractResult<IntoDTOResult> {
        let profit = ProfitRef::try_from(api.addr_validate(&self.loan.profit)?, querier)?;
        // TODO check the address simmilarly to the profit
        let alarms = TimeAlarmsRef::try_from(api.addr_validate(&self.time_alarms)?).unwrap();
        // .expect("Time Alarms is not deployed, or wrong address is passed!");

        lease::execute_deps(
            LeaseFactory {
                form: &self,
                addr,
                start_at,
                api,
            },
            &self.currency,
            lpp,
            profit,
            alarms,
            oracle,
            querier,
        )
    }
}

struct LeaseFactory<'a> {
    form: &'a NewLeaseForm,
    addr: &'a Addr,
    start_at: Timestamp,
    api: &'a dyn Api,
}

impl<'a> WithLeaseDeps for LeaseFactory<'a> {
    type Output = IntoDTOResult;
    type Error = ContractError;

    fn exec<Lpn, Asset, Lpp, Profit, TimeAlarms, Oracle>(
        self,
        lpp: Lpp,
        profit: Profit,
        alarms: TimeAlarms,
        oracle: Oracle,
    ) -> Result<Self::Output, Self::Error>
    where
        Lpn: Currency + Serialize,
        Asset: Currency + Serialize,
        Lpp: LppLenderTrait<Lpn>,
        TimeAlarms: TimeAlarmsTrait,
        Oracle: OracleTrait<Lpn>,
        Profit: ProfitTrait,
    {
        let customer = self.api.addr_validate(&self.form.customer)?;
        //TODO pass in the real amount as Coin<Asset>
        let amount = 0.into();
        let liability = self.form.liability;
        liability.invariant_held()?;

        let loan = Loan::new(
            self.start_at,
            lpp,
            self.form.loan.annual_margin_interest,
            self.form.loan.interest_due_period,
            self.form.loan.grace_period,
            profit,
        )?;

        Ok(Lease::<_, Asset, _, _, _, _>::new(
            self.addr,
            customer,
            amount,
            self.start_at,
            liability,
            loan,
            (alarms, oracle),
        )?
        .into_dto())
    }
}

#[cfg(test)]
mod test {
    use std::any::type_name;

    use cosmwasm_std::{
        from_slice,
        testing::{MockApi, MockQuerier},
        to_binary, Addr, ContractResult, QuerierResult, QuerierWrapper, SystemResult, Timestamp,
        WasmQuery,
    };

    use currency::{lease::Osmo, lpn::Usdc};
    use finance::error::Error as FinanceError;
    use finance::{currency::Currency, duration::Duration};
    use finance::{liability::Liability, percent::Percent};
    use lpp::stub::lender::LppLenderRef;
    use market_price_oracle::{msg::ConfigResponse as OracleConfigResponse, stub::OracleRef};
    use profit::msg::ConfigResponse as ProfitConfigResponse;

    use crate::{
        error::ContractError,
        msg::{LoanForm, NewLeaseForm},
        reply_id::ReplyId,
    };
    const PROFIT_ADDR: &str = "f78wgdw";
    const ORACLE_ADDR: &str = "f383hddnslni";
    type Lpn = Usdc;

    #[test]
    fn amount_to_borrow_broken_invariant() {
        let lpp = "sdgg22d";
        let liability: Liability = from_slice(
            br#"{"initial":40,"healthy":50,"first_liq_warn":52,"second_liq_warn":53,"third_liq_warn":54,"max":54,"recalc_time":36000}"#,
        )
        .unwrap();
        assert!(liability.invariant_held().is_err());
        let lease = NewLeaseForm {
            customer: "ss1s1".into(),
            currency: ToOwned::to_owned(Osmo::SYMBOL),
            liability,
            loan: LoanForm {
                annual_margin_interest: Percent::from_percent(0),
                lpp: lpp.into(),
                interest_due_period: Duration::from_secs(100),
                grace_period: Duration::from_secs(10),
                profit: PROFIT_ADDR.into(),
            },
            time_alarms: "timealarms".into(),
            market_price_oracle: ORACLE_ADDR.into(),
        };
        let api = MockApi::default();

        let mut querier = MockQuerier::default();
        querier.update_wasm(config_req_handler);
        let err = lease
            .into_lease(
                &Addr::unchecked("test"),
                Timestamp::from_nanos(1000),
                &api,
                &QuerierWrapper::new(&querier),
                LppLenderRef::unchecked::<_, Lpn>(lpp, ReplyId::OpenLoanReq.into()),
                OracleRef::unchecked::<_, Lpn>(ORACLE_ADDR),
            )
            .unwrap_err();

        assert_eq!(
            err,
            ContractError::from(FinanceError::BrokenInvariant(
                type_name::<Liability>().into(),
                "Third liquidation % should be < max %".into()
            ))
        );
    }

    fn config_req_handler(request: &WasmQuery) -> QuerierResult {
        match request {
            WasmQuery::Smart {
                contract_addr,
                msg: _,
            } => {
                let resp = if contract_addr == PROFIT_ADDR {
                    to_binary(&ProfitConfigResponse { cadence_hours: 2 })
                } else if contract_addr == ORACLE_ADDR {
                    to_binary(&OracleConfigResponse {
                        base_asset: Lpn::SYMBOL.into(),
                        expected_feeders: Percent::from_percent(50),
                        owner: Addr::unchecked("3d3"),
                        price_feed_period: Duration::from_secs(12),
                    })
                } else {
                    unreachable!()
                }
                .unwrap();
                SystemResult::Ok(ContractResult::Ok(resp))
            }
            &_ => unreachable!(),
        }
    }
}
