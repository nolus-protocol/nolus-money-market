use cosmwasm_std::{Api, Coin, StdResult, Storage, Timestamp};
use cw_storage_plus::Item;
use lpp::stub::Lpp;

use crate::{error::ContractResult, lease::{Lease, self}, loan::Loan, msg::NewLeaseForm};

impl NewLeaseForm {
    const DB_ITEM: Item<'static, NewLeaseForm> = Item::new("lease_form");

    pub(crate) fn amount_to_borrow(&self, downpayment: Coin) -> ContractResult<Coin> {
        assert_eq!(
            downpayment.denom, self.currency,
            "this is a single currency lease version"
        );
        // TODO msg.invariant_held(deps.api) checking invariants including address validity and incorporating the liability and loan form invariants
        self.liability.invariant_held()?;

        Ok(self.liability.init_borrow_amount(downpayment))
    }

    pub(crate) fn into_lease<L>(self, lpp: L, start_at: Timestamp, api: &dyn Api) -> ContractResult<Lease<L>>
    where
        L: Lpp<lease::CURRENCY>,
    {
        let customer = api.addr_validate(&self.customer)?;
        let loan = Loan::open(
            start_at,
            lpp,
            self.loan.annual_margin_interest,
            self.loan.interest_due_period_secs,
            self.loan.grace_period_secs,
        )?;
        Ok(Lease::new(customer, self.currency, self.liability, loan))
    }

    pub fn save(self, storage: &mut dyn Storage) -> StdResult<()> {
        Self::DB_ITEM.save(storage, &self)
    }

    pub fn pull(storage: &mut dyn Storage) -> StdResult<Self> {
        let item = Self::DB_ITEM.load(storage)?;
        Self::DB_ITEM.remove(storage);
        StdResult::Ok(item)
    }
}

#[cfg(test)]
mod test {
    use cosmwasm_std::Coin;
    use finance::{liability::Liability, percent::Percent};

    use crate::msg::{LoanForm, NewLeaseForm};

    #[test]
    fn amount_to_borrow_no_downpayment() {
        let downpayment = Coin::new(0, String::from("YAN"));
        amount_to_borrow_impl(downpayment.clone(), downpayment);
    }

    #[test]
    fn amount_to_borrow_some_downpayment() {
        let downpayment = Coin::new(1000, String::from("YAN"));
        let expected = Coin::new(111, downpayment.denom.clone());
        amount_to_borrow_impl(downpayment, expected);
    }

    #[test]
    #[should_panic]
    fn amount_to_borrow_broken_invariant() {
        let downpayment = Coin::new(0, String::from("YAN"));
        let lease = NewLeaseForm {
            customer: "ss1s1".into(),
            currency: downpayment.denom.clone(),
            liability: Liability::new(Percent::from_percent(10), Percent::from_percent(0), Percent::from_percent(0), 100),
            loan: LoanForm {
                annual_margin_interest: Percent::from_percent(0),
                lpp: "sdgg22d".into(),
                interest_due_period_secs: 100,
                grace_period_secs: 10,
            },
        };
        let _res = lease.amount_to_borrow(downpayment);
    }

    fn amount_to_borrow_impl(downpayment: Coin, expected: Coin) {
        let lease = NewLeaseForm {
            customer: "ss1s1".into(),
            currency: downpayment.denom.clone(),
            liability: Liability::new(Percent::from_percent(10), Percent::from_percent(0), Percent::from_percent(10), 100),
            loan: LoanForm {
                annual_margin_interest: Percent::from_percent(0),
                lpp: "sdgg22d".into(),
                interest_due_period_secs: 100,
                grace_period_secs: 10,
            },
        };
        assert_eq!(expected, lease.amount_to_borrow(downpayment).unwrap());
    }
}
