use cosmwasm_std::{Api, QuerierWrapper, Timestamp};

use finance::duration::Duration;
use lpp::stub::LppRef;

use crate::{
    error::ContractResult, lease::LeaseDTO, loan::LoanDTO, msg::NewLeaseForm, repay_id::ReplyId,
};

impl NewLeaseForm {
    pub(crate) fn into_lease_dto(
        self,
        start_at: Timestamp,
        api: &dyn Api,
        querier: &QuerierWrapper,
    ) -> ContractResult<LeaseDTO> {
        self.liability.invariant_held()?;

        let customer = api.addr_validate(&self.customer)?;

        let lpp = LppRef::try_borrow_from(
            api.addr_validate(&self.loan.lpp)?,
            querier,
            ReplyId::OpenLoanReq.into(),
        )?;

        let loan = LoanDTO::new(
            start_at,
            lpp,
            self.loan.annual_margin_interest,
            Duration::from_secs(self.loan.interest_due_period_secs),
            Duration::from_secs(self.loan.grace_period_secs),
        )?;

        Ok(LeaseDTO::new(
            customer,
            self.currency,
            self.liability,
            loan,
            self.time_alarms,
            self.market_price_oracle,
            self.profit,
        ))
    }
}

#[cfg(test)]
mod test {
    use cosmwasm_std::{
        testing::{MockApi, MockQuerier},
        Addr, QuerierWrapper, Timestamp,
    };

    use finance::{
        currency::{Currency, Nls},
        liability::Liability,
        percent::Percent,
    };

    use crate::msg::{LoanForm, NewLeaseForm};

    #[test]
    #[should_panic]
    fn amount_to_borrow_broken_invariant() {
        let lease = NewLeaseForm {
            customer: "ss1s1".into(),
            currency: ToOwned::to_owned(Nls::SYMBOL),
            liability: Liability::new(
                Percent::from_percent(10),
                Percent::from_percent(0),
                Percent::from_percent(0),
                Percent::default(),
                Percent::default(),
                Percent::default(),
                100,
            ),
            loan: LoanForm {
                annual_margin_interest: Percent::from_percent(0),
                lpp: "sdgg22d".into(),
                interest_due_period_secs: 100,
                grace_period_secs: 10,
            },
            time_alarms: Addr::unchecked("timealarms"),
            market_price_oracle: Addr::unchecked("oracle"),
            profit: Addr::unchecked("profit"),
        };
        let api = MockApi::default();
        let _ = lease.into_lease_dto(
            Timestamp::from_nanos(1000),
            &api,
            &QuerierWrapper::new(&MockQuerier::default()),
        );
    }
}
