use cosmwasm_std::{Addr, QuerierWrapper, StdResult, Storage, SubMsg, Timestamp};
use cw_storage_plus::Item;
use finance::{
    bank::BankAccount,
    coin::Coin,
    currency::{Currency, SymbolOwned},
    liability::Liability,
};
use lpp::stub::Lpp as LppTrait;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::{
    error::{ContractError, ContractResult},
    loan::Loan,
    msg::StateResponse,
};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Lease<Lpn, Lpp> {
    customer: Addr,
    currency: SymbolOwned,
    liability: Liability,
    loan: Loan<Lpn, Lpp>,
}

impl<'a, Lpn, Lpp> Lease<Lpn, Lpp>
where
    Lpn: Currency + Serialize + DeserializeOwned,
    Lpp: LppTrait<Lpn> + Serialize + DeserializeOwned,
{
    const DB_ITEM: Item<'a, Lease<Lpn, Lpp>> = Item::new("lease");

    pub(crate) fn new(
        customer: Addr,
        currency: SymbolOwned,
        liability: Liability,
        loan: Loan<Lpn, Lpp>,
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
        account: &B,
    ) -> ContractResult<SubMsg>
    where
        B: BankAccount,
    {
        let state = self.state(Timestamp::from_nanos(u64::MAX), account, querier, lease)?;
        match state {
            StateResponse::Opened { .. } => ContractResult::Err(ContractError::LoanNotPaid()),
            StateResponse::Paid(..) => {
                let balance = account.balance::<Lpn>()?;
                account
                    .send(balance, &self.customer)
                    .map_err(|err| err.into())
            }
            StateResponse::Closed() => ContractResult::Err(ContractError::LoanClosed()),
        }
    }

    pub(crate) fn repay(
        &mut self,
        payment: Coin<Lpn>,
        by: Timestamp,
        querier: &QuerierWrapper,
        lease: Addr,
    ) -> ContractResult<Option<SubMsg>> {
        assert_eq!(self.currency, Lpn::SYMBOL);
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
        account: &B,
        querier: &QuerierWrapper,
        lease: Addr,
    ) -> ContractResult<StateResponse<Lpn, Lpn>>
    where
        B: BankAccount,
    {
        let lease_amount = account.balance::<Lpn>().map_err(ContractError::from)?;

        if lease_amount.is_zero() {
            Ok(StateResponse::Closed())
        } else {
            let loan_state = self.loan.state(now, querier, lease)?;

            loan_state.map_or_else(
                || Ok(StateResponse::Paid(lease_amount)),
                |state| {
                    Ok(StateResponse::Opened {
                        amount: lease_amount,
                        interest_rate: state.annual_interest,
                        principal_due: state.principal_due,
                        interest_due: state.interest_due,
                    })
                },
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::testing::{mock_dependencies, MockStorage};
    use cosmwasm_std::{Addr, QuerierWrapper, StdResult, SubMsg, Timestamp};
    use finance::currency::Usdc;
    use finance::{
        bank::BankAccount, coin::Coin, currency::Currency, duration::Duration,
        error::Result as FinanceResult, liability::Liability, percent::Percent,
    };
    use lpp::msg::{LoanResponse, QueryLoanResponse};
    use lpp::stub::Lpp;
    use serde::{Deserialize, Serialize};

    use crate::loan::Loan;
    use crate::msg::StateResponse;

    use super::Lease;

    const MARGIN_INTEREST_RATE: Percent = Percent::from_permille(23);
    type TestCurrency = Usdc;

    #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
    pub struct BankStub {
        balance: u128,
    }

    impl BankAccount for BankStub {
        fn balance<C>(&self) -> FinanceResult<Coin<C>>
        where
            C: Currency,
        {
            Ok(Coin::<C>::new(self.balance))
        }

        fn send<C>(&self, _amount: Coin<C>, _to: &Addr) -> FinanceResult<SubMsg> {
            unimplemented!()
        }
    }

    #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
    struct LppLocalStub {
        loan: Option<LoanResponse<TestCurrency>>,
    }

    impl Lpp<TestCurrency> for LppLocalStub {
        fn open_loan_req(&self, _amount: Coin<TestCurrency>) -> StdResult<SubMsg> {
            unimplemented!()
        }

        fn open_loan_resp(&self, _resp: cosmwasm_std::Reply) -> Result<(), String> {
            unimplemented!()
        }

        fn repay_loan_req(&self, _repayment: Coin<TestCurrency>) -> StdResult<SubMsg> {
            todo!()
        }

        fn loan(
            &self,
            _querier: &QuerierWrapper,
            _lease: impl Into<Addr>,
        ) -> StdResult<QueryLoanResponse<TestCurrency>> {
            Result::Ok(self.loan.clone())
        }

        fn loan_outstanding_interest(
            &self,
            _querier: &QuerierWrapper,
            _lease: impl Into<Addr>,
            _by: Timestamp,
        ) -> StdResult<lpp::msg::QueryLoanOutstandingInterestResponse<TestCurrency>> {
            todo!()
        }
    }

    #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
    struct LppLocalStubUnreachable {}

    impl Lpp<TestCurrency> for LppLocalStubUnreachable {
        fn open_loan_req(&self, _amount: Coin<TestCurrency>) -> StdResult<SubMsg> {
            unreachable!()
        }

        fn open_loan_resp(&self, _resp: cosmwasm_std::Reply) -> Result<(), String> {
            unreachable!()
        }

        fn repay_loan_req(&self, _repayment: Coin<TestCurrency>) -> StdResult<SubMsg> {
            unreachable!()
        }

        fn loan(
            &self,
            _querier: &QuerierWrapper,
            _lease: impl Into<Addr>,
        ) -> StdResult<QueryLoanResponse<TestCurrency>> {
            unreachable!()
        }

        fn loan_outstanding_interest(
            &self,
            _querier: &QuerierWrapper,
            _lease: impl Into<Addr>,
            _by: Timestamp,
        ) -> StdResult<lpp::msg::QueryLoanOutstandingInterestResponse<TestCurrency>> {
            unreachable!()
        }
    }

    fn create_lease<L>(lpp_stub: L) -> Lease<TestCurrency, L>
    where
        L: Lpp<TestCurrency>,
    {
        Lease {
            customer: Addr::unchecked("customer"),
            currency: TestCurrency::SYMBOL.to_string(),
            liability: Liability::new(
                Percent::from_percent(65),
                Percent::from_percent(70),
                Percent::from_percent(80),
                10 * 24,
            ),
            loan: Loan::open(
                Timestamp::from_nanos(0),
                lpp_stub,
                MARGIN_INTEREST_RATE,
                Duration::from_secs(0),
                Duration::from_secs(0),
            )
            .unwrap(),
        }
    }

    fn lease_setup(
        loan_response: Option<LoanResponse<TestCurrency>>,
    ) -> Lease<TestCurrency, LppLocalStub> {
        let lpp_stub = LppLocalStub {
            loan: loan_response,
        };

        create_lease(lpp_stub)
    }

    fn create_bank_account(lease_amount: u128) -> BankStub {
        BankStub {
            balance: lease_amount,
        }
    }

    fn request_state(
        lease: Lease<TestCurrency, LppLocalStub>,
        bank_account: &BankStub,
    ) -> StateResponse<TestCurrency, TestCurrency> {
        let mut deps = mock_dependencies();
        lease
            .state(
                Timestamp::from_nanos(0),
                bank_account,
                &deps.as_mut().querier,
                Addr::unchecked("unused"),
            )
            .unwrap()
    }

    #[test]
    fn persist_ok() {
        let mut storage = MockStorage::default();
        let obj = create_lease(LppLocalStub { loan: None });
        let obj_exp = obj.clone();
        obj.store(&mut storage).expect("storing failed");
        let obj_loaded: Lease<TestCurrency, LppLocalStub> =
            Lease::load(&storage).expect("loading failed");
        assert_eq!(obj_exp.customer, obj_loaded.customer);
    }

    #[test]
    // Open state -> Lease's balance in the loan's currency > 0, loan exists in the lpp
    fn state_opened() {
        let lease_amount = 1000;
        let interest_rate = Percent::from_permille(50);
        // LPP loan
        let loan = LoanResponse {
            principal_due: coin(300),
            interest_due: coin(0),
            annual_interest_rate: interest_rate,
            interest_paid: Timestamp::from_nanos(0),
        };

        let bank_account = create_bank_account(lease_amount);
        let lease = lease_setup(Some(loan.clone()));

        let res = request_state(lease, &bank_account);
        let exp = StateResponse::Opened {
            amount: coin(lease_amount),
            interest_rate: MARGIN_INTEREST_RATE.checked_add(interest_rate).unwrap(),
            principal_due: loan.principal_due,
            interest_due: loan.interest_due,
        };

        assert_eq!(exp, res);
    }

    #[test]
    // Paid state -> Lease's balance in the loan's currency > 0, loan doesn't exist in the lpp anymore
    fn state_paid() {
        let lease_amount = 1000;
        let bank_account = create_bank_account(lease_amount);
        let lease = lease_setup(None);

        let res = request_state(lease, &bank_account);
        let exp = StateResponse::Paid(coin(lease_amount));
        assert_eq!(exp, res);
    }

    #[test]
    // Closed state -> Lease's balance in the loan's currency = 0, loan doesn't exist in the lpp anymore
    fn state_closed() {
        let lease_amount = 0;
        let bank_account = create_bank_account(lease_amount);
        let lease = lease_setup(None);

        let res = request_state(lease, &bank_account);
        let exp = StateResponse::Closed();
        assert_eq!(exp, res);
    }

    #[test]
    // Verify that if the Lease's balance is 0, lpp won't be queried for the loan
    fn state_closed_lpp_must_not_be_called() {
        let lpp_stub = LppLocalStubUnreachable {};
        let lease = create_lease(lpp_stub);

        let bank_account = create_bank_account(0);

        let mut deps = mock_dependencies();
        let res = lease
            .state(
                Timestamp::from_nanos(0),
                &bank_account,
                &deps.as_mut().querier,
                Addr::unchecked("unused"),
            )
            .unwrap();

        let exp = StateResponse::Closed();
        assert_eq!(exp, res);
    }

    fn coin(a: u128) -> Coin<TestCurrency> {
        Coin::<TestCurrency>::new(a)
    }
}
