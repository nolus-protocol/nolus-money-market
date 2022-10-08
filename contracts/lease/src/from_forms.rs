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
    ) -> ContractResult<IntoDTOResult> {
        let profit = ProfitRef::try_from(api.addr_validate(&self.loan.profit)?, querier)?;
        // TODO check the address simmilarly to the profit
        let alarms = TimeAlarmsRef::try_from(self.time_alarms.clone()).unwrap();
        // .expect("Time Alarms is not deployed, or wrong address is passed!");
        let oracle = OracleRef::try_from(self.market_price_oracle.clone(), querier)?;
        // .expect("Market Price Oracle is not deployed, or wrong address is passed!");

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
    use cosmwasm_std::{
        from_slice,
        testing::{MockApi, MockQuerier},
        Addr, QuerierWrapper, Timestamp,
    };

    use finance::{currency::Currency, duration::Duration, test::currency::Nls};
    use finance::{liability::Liability, percent::Percent};
    use lpp::stub::lender::LppLenderRef;

    use crate::{
        error::ContractError,
        msg::{LoanForm, NewLeaseForm},
        reply_id::ReplyId,
    };

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
            currency: ToOwned::to_owned(Nls::SYMBOL),
            liability,
            loan: LoanForm {
                annual_margin_interest: Percent::from_percent(0),
                lpp: lpp.into(),
                interest_due_period: Duration::from_secs(100),
                grace_period: Duration::from_secs(10),
                profit: "profit".into(),
            },
            time_alarms: Addr::unchecked("timealarms"),
            market_price_oracle: Addr::unchecked("oracle"),
        };
        let api = MockApi::default();
        let err = lease
            .into_lease(
                &Addr::unchecked("test"),
                Timestamp::from_nanos(1000),
                &api,
                &QuerierWrapper::new(&MockQuerier::default()),
                LppLenderRef::unchecked::<_, Nls>(lpp, ReplyId::OpenLoanReq.into()),
            )
            .unwrap_err();

        // TODO assert_eq!(err, ContractError::FinanceError(..)));
        assert!(matches!(dbg!(err), ContractError::ProfitError(..)));
    }
}
