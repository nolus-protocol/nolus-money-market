use cosmwasm_std::{Api, QuerierWrapper, Timestamp};

use lpp::stub::lender::LppLenderRef;
use profit::stub::ProfitRef;

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

        let lpp = LppLenderRef::try_new(
            api.addr_validate(&self.loan.lpp)?,
            querier,
            ReplyId::OpenLoanReq.into(),
        )?;

        let profit = ProfitRef::try_from(api.addr_validate(&self.loan.profit)?, querier)?;

        let loan = LoanDTO::new(
            start_at,
            lpp,
            self.loan.annual_margin_interest,
            self.loan.interest_due_period,
            self.loan.grace_period,
            profit,
        )?;

        Ok(LeaseDTO::new(
            customer,
            self.currency,
            self.liability,
            loan,
            self.time_alarms,
            self.market_price_oracle,
        ))
    }
}

#[cfg(test)]
mod test {
    use cosmwasm_std::{
        testing::{MockApi, MockQuerier},
        Addr, QuerierWrapper, Timestamp,
    };

    use finance::{currency::Currency, duration::Duration, test::currency::Nls};
    use finance::{liability::Liability, percent::Percent};

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
                interest_due_period: Duration::from_secs(100),
                grace_period: Duration::from_secs(10),
                profit: "profit".into(),
            },
            time_alarms: Addr::unchecked("timealarms"),
            market_price_oracle: Addr::unchecked("oracle"),
        };
        let api = MockApi::default();
        let _ = lease.into_lease_dto(
            Timestamp::from_nanos(1000),
            &api,
            &QuerierWrapper::new(&MockQuerier::default()),
        );
    }
}
