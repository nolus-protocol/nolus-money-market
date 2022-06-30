use cosmwasm_std::{Addr, Coin, QuerierWrapper, StdResult, Storage, SubMsg, Timestamp};
use cw_storage_plus::Item;
use finance::{
    bank::BankAccount,
    coin_legacy::to_cosmwasm,
    currency::{SymbolOwned, Usdc},
    liability::Liability,
};
use lpp::stub::Lpp;
use serde::{Deserialize, Serialize};

use crate::{
    error::{ContractError, ContractResult},
    loan::Loan,
    msg::StateResponse,
};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Lease<L> {
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
    ) -> ContractResult<StateResponse>
    where
        B: BankAccount,
    {
        let lease_amount = to_cosmwasm(account.balance::<Usdc>().map_err(ContractError::from)?);

        if lease_amount.amount.is_zero() {
            Ok(StateResponse::Closed())
        } else {
            let loan_state = self.loan.state(now, querier, lease)?;

            loan_state.map_or_else(
                || Ok(StateResponse::Paid(lease_amount.clone())),
                |state| {
                    Ok(StateResponse::Opened {
                        amount: lease_amount.clone(),
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
    use cosmwasm_std::{coin, Addr, Coin as CWCoin, QuerierWrapper, StdResult, SubMsg, Timestamp};
    use finance::{
        bank::BankAccount,
        coin::Coin,
        coin_legacy::from_cosmwasm,
        currency::{Currency, Usdc},
        error::Result as FinanceResult,
        liability::Liability,
        percent::Percent,
    };
    use lpp::msg::{LoanResponse, QueryLoanResponse};
    use lpp::stub::Lpp;
    use serde::{Deserialize, Serialize};

    use crate::loan::Loan;
    use crate::msg::StateResponse;

    use super::Lease;

    const DENOM: &str = Usdc::SYMBOL;
    const MARGIN_INTEREST_RATE: Percent = Percent::from_permille(23);

    #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
    pub struct BankStub {
        balance: CWCoin,
    }

    impl BankAccount for BankStub {
        fn balance<C>(&self) -> FinanceResult<Coin<C>>
        where
            C: Currency,
        {
            from_cosmwasm(self.balance.clone())
        }

        fn send<C>(&self, _amount: Coin<C>, _to: &Addr) -> FinanceResult<SubMsg> {
            unimplemented!()
        }
    }

    #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
    struct LppLocalStub {
        loan: Option<LoanResponse>,
    }

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
            Result::Ok(self.loan.clone())
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

    #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
    struct LppLocalStubUnreachable {}

    impl Lpp<super::Currency> for LppLocalStubUnreachable {
        fn open_loan_req(&self, _amount: Coin<super::Currency>) -> StdResult<SubMsg> {
            unreachable!()
        }

        fn open_loan_resp(&self, _resp: cosmwasm_std::Reply) -> Result<(), String> {
            unreachable!()
        }

        fn repay_loan_req(&self, _repayment: Coin<super::Currency>) -> StdResult<SubMsg> {
            unreachable!()
        }

        fn loan(
            &self,
            _querier: &QuerierWrapper,
            _lease: impl Into<Addr>,
        ) -> StdResult<QueryLoanResponse> {
            unreachable!()
        }

        fn loan_outstanding_interest(
            &self,
            _querier: &QuerierWrapper,
            _lease: impl Into<Addr>,
            _by: Timestamp,
        ) -> StdResult<lpp::msg::QueryLoanOutstandingInterestResponse> {
            unreachable!()
        }
    }

    fn create_lease<L>(lpp_stub: L) -> Lease<L>
    where
        L: Lpp<super::Currency>,
    {
        Lease {
            customer: Addr::unchecked("customer"),
            currency: DENOM.to_string(),
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
                0,
                0,
            )
            .unwrap(),
        }
    }

    fn lease_setup(loan_response: Option<LoanResponse>) -> Lease<LppLocalStub> {
        let lpp_stub = LppLocalStub {
            loan: loan_response,
        };

        create_lease(lpp_stub)
    }

    fn create_bank_account(lease_amount: u128) -> BankStub {
        BankStub {
            balance: coin(lease_amount, DENOM),
        }
    }

    fn request_state(lease: Lease<LppLocalStub>, bank_account: BankStub) -> StateResponse {
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
        let obj_loaded: Lease<LppLocalStub> = Lease::load(&storage).expect("loading failed");
        assert_eq!(obj_exp.customer, obj_loaded.customer);
    }

    #[test]
    // Open state -> Lease's balance in the loan's currency > 0, loan exists in the lpp
    fn state_opened() {
        let lease_amount = 1000;
        let interest_rate = Percent::from_permille(50);
        // LPP loan
        let loan = LoanResponse {
            principal_due: coin(300, DENOM),
            interest_due: coin(0, DENOM),
            annual_interest_rate: interest_rate,
            interest_paid: Timestamp::from_nanos(0),
        };

        let bank_account = create_bank_account(lease_amount);
        let lease = lease_setup(Some(loan.clone()));

        let res = request_state(lease, bank_account);
        let exp = StateResponse::Opened {
            amount: coin(lease_amount, DENOM),
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

        let res = request_state(lease, bank_account);
        let exp = StateResponse::Paid(coin(lease_amount, DENOM));
        assert_eq!(exp, res);
    }

    #[test]
    // Closed state -> Lease's balance in the loan's currency = 0, loan doesn't exist in the lpp anymore
    fn state_closed() {
        let lease_amount = 0;
        let bank_account = create_bank_account(lease_amount);
        let lease = lease_setup(None);

        let res = request_state(lease, bank_account);
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
                bank_account,
                &deps.as_mut().querier,
                Addr::unchecked("unused"),
            )
            .unwrap();

        let exp = StateResponse::Closed();
        assert_eq!(exp, res);
    }
}
