use cosmwasm_std::{Api, StdResult, Storage, Timestamp};
use cw_storage_plus::Item;
use finance::{coin::Coin, currency::Currency};
use lpp::stub::Lpp;
use serde::{de::DeserializeOwned, Serialize};

use crate::{error::ContractResult, lease::Lease, loan::Loan, msg::NewLeaseForm};

impl NewLeaseForm {
    const DB_ITEM: Item<'static, Self> = Item::new("lease_form");

    pub(crate) fn amount_to_borrow<Lpn>(&self, downpayment: Coin<Lpn>) -> ContractResult<Coin<Lpn>>
    where
        Lpn: Currency,
    {
        assert_eq!(
            Lpn::SYMBOL,
            self.currency,
            "[Single currency version] The LPN '{}' should match the currency of the lease '{}'",
            Lpn::SYMBOL,
            self.currency
        );
        // TODO msg.invariant_held(deps.api) checking invariants including address validity and incorporating the liability and loan form invariants
        self.liability.invariant_held()?;

        Ok(self.liability.init_borrow_amount(downpayment))
    }

    pub(crate) fn into_lease<C, L>(
        self,
        lpp: L,
        start_at: Timestamp,
        api: &dyn Api,
    ) -> ContractResult<Lease<C, L>>
    where
        C: Currency + Serialize + DeserializeOwned,
        L: Lpp<C> + Serialize + DeserializeOwned,
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
    use std::fmt::Debug;

    use finance::{
        coin::Coin,
        currency::{Currency, Nls, Usdc},
        liability::Liability,
        percent::Percent,
    };

    use crate::msg::{LoanForm, NewLeaseForm};

    #[test]
    fn amount_to_borrow_no_downpayment() {
        let downpayment = Coin::<Usdc>::new(0);
        amount_to_borrow_impl(downpayment, downpayment);
    }

    #[test]
    fn amount_to_borrow_some_downpayment() {
        let downpayment = Coin::<Nls>::new(1000);
        let expected = Coin::<Nls>::new(111);
        amount_to_borrow_impl(downpayment, expected);
    }

    #[test]
    #[should_panic]
    fn amount_to_borrow_broken_invariant() {
        let downpayment = Coin::<Nls>::new(0);
        let lease = NewLeaseForm {
            customer: "ss1s1".into(),
            currency: Nls::SYMBOL.to_owned(),
            liability: Liability::new(
                Percent::from_percent(10),
                Percent::from_percent(0),
                Percent::from_percent(0),
                100,
            ),
            loan: LoanForm {
                annual_margin_interest: Percent::from_percent(0),
                lpp: "sdgg22d".into(),
                interest_due_period_secs: 100,
                grace_period_secs: 10,
            },
        };
        let _res = lease.amount_to_borrow(downpayment);
    }

    fn amount_to_borrow_impl<C>(downpayment: Coin<C>, expected: Coin<C>)
    where
        C: Currency + Debug,
    {
        let lease = NewLeaseForm {
            customer: "ss1s1".into(),
            currency: C::SYMBOL.to_owned(),
            liability: Liability::new(
                Percent::from_percent(10),
                Percent::from_percent(0),
                Percent::from_percent(10),
                100,
            ),
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
