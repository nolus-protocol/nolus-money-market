use cosmwasm_std::{Api, QuerierWrapper, Timestamp};

use finance::duration::Duration;
use lpp::stub::LppRef;
use market_price_oracle::stub::OracleRef;

use crate::{
    constants::ReplyId, error::ContractResult, lease::LeaseDTO, loan::LoanDTO, msg::NewLeaseForm,
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

        let lpp = LppRef::try_from(
            self.loan.lpp.clone(),
            api,
            querier,
            ReplyId::OpenLoanReq as u64,
        )?;

        let oracle = OracleRef::try_from(self.market_price_oracle.to_string(), api, querier)?;

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
            oracle,
        ))
    }
}

#[cfg(test)]
mod test {
    use cosmwasm_std::{
        Addr,
        QuerierWrapper, testing::{MockApi, MockQuerier}, Timestamp,
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
