use cosmwasm_std::{Addr, Coin, QuerierWrapper, StdResult, Storage, SubMsg, Timestamp};
use cw_storage_plus::Item;
use finance::{
    bank::BankAccount,
    coin_legacy::to_cosmwasm,
    currency::{Usdc, SymbolOwned},
    liability::Liability,
};
use lpp::stub::Lpp;
use serde::{Deserialize, Serialize};

use crate::{
    error::{ContractError, ContractResult},
    loan::{Loan, State as LoanState},
    msg::State,
};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Lease<L>
{
    customer: Addr,
    currency: SymbolOwned,
    liability: Liability,
    loan: Loan<L>,
}

//TODO transform it into a Lease type
pub type Currency = Usdc;

impl<'a, L> Lease<L>
where
    L: Lpp<Currency>,
{
    const DB_ITEM: Item<'a, Lease<L>> = Item::new("lease");

    pub(crate) fn new(
        customer: Addr,
        currency: SymbolOwned,
        liability: Liability,
        loan: Loan<L>,
    ) -> Self {
        Self {
            customer,
            currency,
            liability,
            loan,
        }
    }

    pub(crate) fn close<B>(
        &self,
        lease: Addr,
        querier: &QuerierWrapper,
        account: B,
    ) -> ContractResult<SubMsg>
    where
        B: BankAccount,
    {
        if !self.loan.closed(querier, lease)? {
            return ContractResult::Err(ContractError::LoanNotPaid {});
        }
        let balance = account.balance::<Currency>()?;
        account
            .send(balance, &self.customer)
            .map_err(|err| err.into())
    }

    pub(crate) fn repay(
        &mut self,
        payment: Coin,
        by: Timestamp,
        querier: &QuerierWrapper,
        lease: Addr,
    ) -> ContractResult<Option<SubMsg>> {
        debug_assert_eq!(self.currency, payment.denom);
        self.loan.repay(payment, by, querier, lease)
    }

    pub(crate) fn store(self, storage: &mut dyn Storage) -> StdResult<()> {
        Lease::DB_ITEM.save(storage, &self)
    }

    pub(crate) fn load(storage: &dyn Storage) -> StdResult<Self> {
        Lease::DB_ITEM.load(storage)
    }

    pub(crate) fn owned_by(&self, addr: &Addr) -> bool {
        &self.customer == addr
    }

    pub(crate) fn state<B>(
        &self,
        now: Timestamp,
        account: B,
        querier: &QuerierWrapper,
        lease: Addr,
    ) -> ContractResult<Option<State>>
    where
        B: BankAccount,
    {
        let loan_state = self.loan.state(now, querier, lease)?;
        loan_state
            .map(|open_state| self.merge_state(account, open_state))
            .transpose()
    }

    fn merge_state<B>(&self, account: B, loan_state: LoanState) -> ContractResult<State>
    where
        B: BankAccount,
    {
        let lease_amount = account.balance::<Usdc>().map_err(ContractError::from)?;

        Ok(State {
            amount: to_cosmwasm(lease_amount),
            annual_interest: loan_state.annual_interest,
            principal_due: loan_state.principal_due,
            interest_due: loan_state.interest_due,
        })
    }
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::{testing::MockStorage, Addr, QuerierWrapper, StdResult, SubMsg, Timestamp};
    use finance::{coin::Coin, liability::Liability, percent::Percent};
    use lpp::{msg::QueryLoanResponse, stub::Lpp};
    use serde::{Deserialize, Serialize};

    use crate::loan::Loan;

    use super::Lease;

    #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
    struct LppLocalStub {}
    impl Lpp<super::Currency> for LppLocalStub {
        fn open_loan_req(&self, _amount: Coin<super::Currency>) -> StdResult<SubMsg> {
            unimplemented!()
        }

        fn open_loan_resp(&self, _resp: cosmwasm_std::Reply) -> Result<(), String> {
            unimplemented!()
        }

        fn repay_loan_req(&self, _repayment: Coin<super::Currency>) -> StdResult<SubMsg> {
            todo!()
        }
        fn loan(
            &self,
            _querier: &QuerierWrapper,
            _lease: impl Into<Addr>,
        ) -> StdResult<QueryLoanResponse> {
            todo!()
        }

        fn loan_outstanding_interest(
            &self,
            _querier: &QuerierWrapper,
            _lease: impl Into<Addr>,
            _by: Timestamp,
        ) -> StdResult<lpp::msg::QueryLoanOutstandingInterestResponse> {
            todo!()
        }
    }

    #[test]
    fn persist_ok() {
        let mut storage = MockStorage::default();
        let obj = Lease {
            customer: Addr::unchecked("test"),
            currency: "UST".to_owned(),
            liability: Liability::new(
                Percent::from_percent(65),
                Percent::from_percent(5),
                Percent::from_percent(10),
                10 * 24,
            ),
            loan: Loan::open(
                Timestamp::default(),
                LppLocalStub {},
                Percent::from_percent(23),
                100,
                10,
            )
            .unwrap(),
        };
        let obj_exp = obj.clone();
        obj.store(&mut storage).expect("storing failed");
        let obj_loaded: Lease<LppLocalStub> = Lease::load(&storage).expect("loading failed");
        assert_eq!(obj_exp.customer, obj_loaded.customer);
    }
}
