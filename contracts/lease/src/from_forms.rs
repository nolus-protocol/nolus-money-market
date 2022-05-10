use cosmwasm_std::{Api, Coin};
use lpp::stub::LppStub;

use crate::{
    error::ContractResult,
    lease::Lease,
    loan::Loan,
    opening::{LoanForm, NewLeaseForm},
};

impl LoanForm {
    pub fn request(self, borrow: Coin, api: &dyn Api) -> ContractResult<Loan<LppStub>> {
        let lpp = lpp::stub::LppStub::try_from(self.lpp, api)?;
        // debug_assert_eq!(lpp_stub.currency(), borrow.denom); TODO
        Loan::open(
            borrow,
            lpp,
            self.annual_margin_interest_permille,
            self.interest_due_period_secs,
            self.grace_period_secs,
        )
    }
}

impl NewLeaseForm {
    pub fn open_lease(self, downpayment: Coin, api: &dyn Api) -> ContractResult<Lease<LppStub>> {
        assert_eq!(
            &downpayment.denom, &self.currency,
            "this is a single currency lease version"
        );
        self.liability.invariant_held()?;
        let customer = api.addr_validate(&self.customer)?;
        let borrow = self.liability.init_borrow_amount(downpayment.amount.into());
        let borrow_coin = Coin::new(borrow.into(), self.currency.clone());
        Ok(Lease::new(
            customer,
            self.currency,
            self.liability,
            self.loan.request(borrow_coin, api)?,
        ))
    }
}
