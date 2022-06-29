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
    msg::{Denom, StateResponse},
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
    use cosmwasm_std::testing::{mock_dependencies, mock_env, MockApi, MockQuerier, MockStorage};
    use cosmwasm_std::{
        Addr, Coin as CWCoin, DepsMut, Env, MemoryStorage, OwnedDeps, QuerierWrapper, StdResult,
        SubMsg, Timestamp,
    };
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

        fn send<Usdc>(&self, _amount: Coin<Usdc>, _to: &Addr) -> FinanceResult<SubMsg> {
            unimplemented!()
        }
    }

    #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
    struct LppLocalStub {
        addr: Addr,
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
                LppLocalStub {
                    addr: Addr::unchecked("lpp"),
                    loan: None,
                },
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

    fn lease_setup(
        loan_response: Option<LoanResponse>,
        lease_amount: u128,
    ) -> (
        Lease<LppLocalStub>,
        Env,
        BankStub,
        OwnedDeps<MemoryStorage, MockApi, MockQuerier>,
    ) {
        let deps = mock_dependencies();
        let mut env = mock_env();
        env.block.time = Timestamp::from_nanos(0);

        let bank_account = get_bank_account(lease_amount);

        let lpp_stub = LppLocalStub {
            addr: Addr::unchecked("lpp"),
            loan: loan_response,
        };

        let lease = Lease {
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
                Percent::from_permille(23),
                0,
                0,
            )
            .unwrap(),
        };

        (lease, env, bank_account, deps)
    }

    fn get_bank_account(lease_amount: u128) -> BankStub {
        BankStub {
            balance: CWCoin::new(lease_amount, DENOM),
        }
    }

    fn request_state(
        lease: Lease<LppLocalStub>,
        env: Env,
        bank_account: BankStub,
        deps: &DepsMut,
    ) -> StateResponse {
        lease
            .state(
                env.block.time,
                bank_account,
                &deps.querier,
                Addr::unchecked("unused"),
            )
            .unwrap()
    }

    #[test]
    fn state_opened() {
        let lease_amount = 1000;
        // LPP loan
        let loan = LoanResponse {
            principal_due: CWCoin::new(300, DENOM),
            interest_due: CWCoin::new(0, DENOM),
            annual_interest_rate: Percent::from_permille(50),
            interest_paid: Timestamp::from_nanos(0),
        };

        let (lease, env, bank_account, mut deps) = lease_setup(Some(loan.clone()), lease_amount);

        let res = request_state(lease, env, bank_account, &deps.as_mut());
        let exp = StateResponse::Opened {
            amount: CWCoin::new(lease_amount, DENOM),
            interest_rate: Percent::from_permille(73),
            principal_due: loan.principal_due,
            interest_due: loan.interest_due,
        };
        assert_eq!(
            exp, res,
            "EXPECTED =======> {:#?} \n ACTUAL =======> {:#?}",
            exp, res
        );
    }

    #[test]
    fn state_paid() {
        let lease_amount = 1000;
        let (lease, env, bank_account, mut deps) = lease_setup(None, lease_amount);

        let res = request_state(lease, env, bank_account, &deps.as_mut());
        let exp = StateResponse::Paid(CWCoin::new(lease_amount, DENOM));
        assert_eq!(
            exp, res,
            "EXPECTED =======> {:#?} \n ACTUAL =======> {:#?}",
            exp, res
        );
    }

    #[test]
    fn state_closed_no_loan() {
        let lease_amount = 0;
        let (lease, env, bank_account, mut deps) = lease_setup(None, lease_amount);

        let res = request_state(lease, env, bank_account, &deps.as_mut());
        assert_eq!(
            exp, res,
            "EXPECTED =======> {:#?} \n ACTUAL =======> {:#?}",
            exp, res
        );
        let exp = StateResponse::Closed();
    }

    #[test]
    fn state_closed_loan_is_ignored() {
        let lease_amount = 0;
        let (lease, env, bank_account, mut deps) = lease_setup(
            // This loan would never be requested
            Some(LoanResponse {
                principal_due: CWCoin::new(0, DENOM),
                interest_due: CWCoin::new(0, DENOM),
                annual_interest_rate: Percent::from_permille(50),
                interest_paid: Timestamp::from_nanos(0),
            }),
            lease_amount,
        );

        let res = request_state(lease, env, bank_account, &deps.as_mut());
        assert_eq!(
            exp, res,
            "EXPECTED =======> {:#?} \n ACTUAL =======> {:#?}",
            exp, res
        );
        let exp = StateResponse::Closed();
    }
}
