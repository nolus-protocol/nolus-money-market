use cosmwasm_std::{Api, Coin, StdResult, Storage};
use cw_storage_plus::Item;
use lpp::stub::Lpp;

use crate::{
    error::ContractResult,
    lease::Lease,
    loan::Loan,
    opening::{LoanForm, NewLeaseForm},
};

impl LoanForm {
    pub fn into_loan<L>(self, lpp: L) -> ContractResult<Loan<L>>
    where
        L: Lpp,
    {
        Loan::open(
            lpp,
            self.annual_margin_interest_permille,
            self.interest_due_period_secs,
            self.grace_period_secs,
        )
    }
}

impl NewLeaseForm {
    const DB_ITEM: Item<'static, NewLeaseForm> = Item::new("lease_form");

    pub(crate) fn amount_to_borrow(&self, downpayment: &Coin) -> ContractResult<Coin> {
        assert_eq!(
            downpayment.denom, self.currency,
            "this is a single currency lease version"
        );
        // TODO msg.invariant_held(deps.api) checking invariants including address validity and incorporating the liability and loan form invariants
        self.liability.invariant_held()?;

        // TODO provide Percent.of(&self, Coin) -> Coin and get rid of Amount
        let borrow = self.liability.init_borrow_amount(downpayment.amount.into());
        Ok(Coin::new(borrow.into(), self.currency.clone()))
    }

    pub(crate) fn into_lease<L>(self, lpp: L, api: &dyn Api) -> ContractResult<Lease<L>>
    where
        L: Lpp,
    {
        let customer = api.addr_validate(&self.customer)?;
        Ok(Lease::new(
            customer,
            self.currency,
            self.liability,
            self.loan.into_loan(lpp)?,
        ))
    }

    pub fn store(self, storage: &mut dyn Storage) -> StdResult<()> {
        Self::DB_ITEM.save(storage, &self)
    }

    pub fn load(storage: &dyn Storage) -> StdResult<Self> {
        Self::DB_ITEM.load(storage)
    }
}

#[cfg(test)]
mod test {
    use cosmwasm_std::Coin;

    use crate::{
        liability::Liability,
        opening::{LoanForm, NewLeaseForm},
    };

    #[test]
    fn amount_to_borrow_no_downpayment() {
        let downpayment = Coin::new(0, String::from("YAN"));
        amount_to_borrow_impl(&downpayment, &downpayment);
    }

    #[test]
    fn amount_to_borrow_some_downpayment() {
        let downpayment = Coin::new(1000, String::from("YAN"));
        let expected = Coin::new(111, downpayment.denom.clone());
        amount_to_borrow_impl(&downpayment, &expected);
    }

    #[test]
    #[should_panic]
    fn amount_to_borrow_broken_invariant() {
        let downpayment = Coin::new(0, String::from("YAN"));
        let lease = NewLeaseForm {
            customer: "ss1s1".into(),
            currency: downpayment.denom.clone(),
            liability: Liability::new(10, 0, 0, 100),
            loan: LoanForm {
                annual_margin_interest_permille: 0,
                lpp: "sdgg22d".into(),
                interest_due_period_secs: 100,
                grace_period_secs: 10,
            }
        };
        let _res = lease.amount_to_borrow(&downpayment);
    }

    fn amount_to_borrow_impl(downpayment: &Coin, expected: &Coin) {
        let lease = NewLeaseForm {
            customer: "ss1s1".into(),
            currency: downpayment.denom.clone(),
            liability: Liability::new(10, 0, 10, 100),
            loan: LoanForm {
                annual_margin_interest_permille: 0,
                lpp: "sdgg22d".into(),
                interest_due_period_secs: 100,
                grace_period_secs: 10,
            }
        };
        assert_eq!(expected, &lease.amount_to_borrow(downpayment).unwrap());
    }
}
