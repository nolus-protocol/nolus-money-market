mod dto;
pub(super) use dto::LeaseDTO;
mod factory;
pub(crate) mod open_request;

use cosmwasm_std::{Addr, QuerierWrapper, Reply, Timestamp};
use finance::{
    coin::Coin,
    currency::{Currency, SymbolOwned},
    liability::Liability,
};
use lpp::stub::Lpp as LppTrait;
use platform::batch::Emitter;
use platform::{
    bank::{BankAccount, BankAccountView},
    batch::Batch,
};
use serde::Serialize;

use crate::{
    error::{ContractError, ContractResult},
    loan::{Loan, Receipt},
    msg::StateResponse,
};

use self::{
    open_request::Result as OpenRequestResult,
    factory::Factory,
};

pub trait WithLease {
    type Output;
    type Error;

    fn exec<Lpn, Lpp>(self, lease: Lease<Lpn, Lpp>) -> Result<Self::Output, Self::Error>
    where
        Lpn: Currency + Serialize,
        Lpp: LppTrait<Lpn>;

    fn unknown_lpn(self, symbol: SymbolOwned) -> Result<Self::Output, Self::Error>;
}

pub fn execute<L, O, E>(dto: LeaseDTO, cmd: L, querier: &QuerierWrapper) -> Result<O, E>
where
    L: WithLease<Output = O, Error = E>,
{
    let lpp = dto.loan.lpp().clone();
    lpp.execute(Factory::new(cmd, dto), querier)
}

pub struct Lease<Lpn, Lpp> {
    customer: Addr,
    currency: SymbolOwned,
    liability: Liability,
    loan: Loan<Lpn, Lpp>,
}

impl<Lpn, Lpp> Lease<Lpn, Lpp>
where
    Lpn: Currency,
    Lpp: LppTrait<Lpn>,
{
    pub(super) fn from_dto(dto: LeaseDTO, lpp: Lpp) -> Self {
        assert_eq!(
            Lpn::SYMBOL,
            dto.currency,
            "[Single currency version] The LPN '{}' should match the currency of the lease '{}'",
            Lpn::SYMBOL,
            dto.currency
        );

        Self {
            customer: dto.customer,
            currency: dto.currency,
            liability: dto.liability,
            loan: Loan::from_dto(dto.loan, lpp),
        }
    }

    pub(super) fn into_dto(self) -> (LeaseDTO, Lpp) {
        let (loan_dto, lpp) = self.loan.into_dto();
        (
            LeaseDTO::new(self.customer, self.currency, self.liability, loan_dto),
            lpp,
        )
    }

    pub(crate) fn owned_by(&self, addr: &Addr) -> bool {
        &self.customer == addr
    }

    pub(crate) fn open_loan_req(self, lease: Addr, downpayment: Coin<Lpn>) -> ContractResult<OpenRequestResult<Lpn>> {
        // TODO add a type parameter to this function to designate the downpayment currency
        // TODO query the market price oracle to get the price of the downpayment currency to LPN
        //  and calculate `downpayment` in LPN
        let borrow = self.liability.init_borrow_amount(downpayment);

        let result = self.loan.open_loan_req(lease, borrow)?;

        Ok(OpenRequestResult {
            batch: result.batch,
            customer: self.customer,
            annual_interest_rate: result.annual_interest_rate,
            annual_interest_rate_margin: result.annual_interest_rate_margin,
            currency: self.currency,
            loan_pool_id: result.loan_pool_id,
            loan_amount: borrow,
        })
    }

    pub(crate) fn open_loan_resp(self, resp: Reply) -> ContractResult<Batch> {
        // use cw_utils::parse_reply_instantiate_data;
        self.loan.open_loan_resp(resp)
    }

    // TODO add the lease address as a field in Lease<>
    //  and populate it on LeaseDTO.execute as LeaseFactory
    pub(crate) fn close<B>(self, lease: Addr, mut account: B) -> ContractResult<Batch>
    where
        B: BankAccount,
    {
        let state = self.state(Timestamp::from_nanos(u64::MAX), &account, lease)?;
        match state {
            StateResponse::Opened { .. } => Err(ContractError::LoanNotPaid()),
            StateResponse::Paid(..) => {
                let balance = account.balance::<Lpn>()?;
                account.send(balance, &self.customer);

                Ok(account.into())
            }
            StateResponse::Closed() => Err(ContractError::LoanClosed()),
        }
    }

    pub(crate) fn repay(
        &mut self,
        payment: Coin<Lpn>,
        by: Timestamp,
        lease: Addr,
    ) -> ContractResult<Receipt<Lpn>> {
        assert_eq!(self.currency, Lpn::SYMBOL);
        self.loan.repay(payment, by, lease)
    }

    pub(crate) fn state<B>(
        &self,
        now: Timestamp,
        account: &B,
        lease: Addr,
    ) -> ContractResult<StateResponse<Lpn, Lpn>>
    where
        B: BankAccountView,
    {
        let lease_amount = account.balance::<Lpn>().map_err(ContractError::from)?;

        if lease_amount.is_zero() {
            Ok(StateResponse::Closed())
        } else {
            let loan_state = self.loan.state(now, lease)?;

            loan_state.map_or(
                Ok(StateResponse::Paid(lease_amount)),
                |state| {
                    Ok(StateResponse::Opened {
                        amount: lease_amount,
                        interest_rate: state.annual_interest,
                        interest_rate_margin: state.annual_interest_margin,
                        principal_due: state.principal_due,
                        previous_margin_due: state.previous_margin_interest_due,
                        previous_interest_due: state.previous_interest_due,
                        current_margin_due: state.current_margin_interest_due,
                        current_interest_due: state.current_interest_due,
                    })
                },
            )
        }
    }
}

#[cfg(test)]
mod tests {

    use cosmwasm_std::{Addr, Timestamp};
    use finance::currency::{Nls, Usdc};
    use finance::{
        coin::Coin, currency::Currency, duration::Duration, liability::Liability, percent::Percent,
    };
    use lpp::error::ContractError as LppError;
    use lpp::msg::{LoanResponse, QueryLoanResponse};
    use lpp::stub::{Lpp, LppRef};

    use platform::bank::BankAccountView;
    use platform::batch::Batch;
    use platform::error::Result as PlatformResult;
    use serde::{Deserialize, Serialize};

    use crate::loan::{Loan, LoanDTO};
    use crate::msg::StateResponse;

    use super::Lease;

    const MARGIN_INTEREST_RATE: Percent = Percent::from_permille(23);
    const LEASE_START: Timestamp = Timestamp::from_nanos(100);
    const LEASE_STATE_AT: Timestamp = Timestamp::from_nanos(200);
    type TestCurrency = Usdc;
    type LppResult<T> = Result<T, LppError>;

    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    pub struct BankStub {
        balance: u128,
    }

    impl BankAccountView for BankStub {
        fn balance<C>(&self) -> PlatformResult<Coin<C>>
        where
            C: Currency,
        {
            Ok(Coin::<C>::new(self.balance))
        }
    }

    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    struct LppLocalStub {
        loan: Option<LoanResponse<TestCurrency>>,
    }

    // TODO define a MockLpp trait to avoid implementing Lpp-s from scratch
    impl Lpp<TestCurrency> for LppLocalStub {
        fn id(&self) -> Addr {
            Addr::unchecked("0123456789ABDEF0123456789ABDEF0123456789ABDEF0123456789ABDEF")
        }

        fn open_loan_req(&mut self, _amount: Coin<TestCurrency>) -> LppResult<()> {
            unreachable!()
        }

        fn open_loan_resp(&self, _resp: cosmwasm_std::Reply) -> LppResult<()> {
            unreachable!()
        }

        fn repay_loan_req(&mut self, _repayment: Coin<TestCurrency>) -> LppResult<()> {
            unreachable!()
        }

        fn loan(&self, _lease: impl Into<Addr>) -> LppResult<QueryLoanResponse<TestCurrency>> {
            Ok(self.loan.clone())
        }

        fn loan_outstanding_interest(
            &self,
            _lease: impl Into<Addr>,
            _by: Timestamp,
        ) -> LppResult<lpp::msg::QueryLoanOutstandingInterestResponse<TestCurrency>> {
            unreachable!()
        }

        fn quote(&self, _amount: Coin<TestCurrency>) -> LppResult<lpp::msg::QueryQuoteResponse> {
            unreachable!()
        }

        fn lpp_balance(&self) -> LppResult<lpp::msg::LppBalanceResponse<TestCurrency>> {
            unreachable!()
        }

        fn nlpn_price(&self) -> LppResult<lpp::msg::PriceResponse<TestCurrency>> {
            unreachable!()
        }

        fn config(&self) -> LppResult<lpp::msg::QueryConfigResponse> {
            unreachable!()
        }

        fn nlpn_balance(&self, _lender: impl Into<Addr>) -> LppResult<lpp::msg::BalanceResponse> {
            unreachable!()
        }

        fn rewards(&self, _lender: impl Into<Addr>) -> LppResult<lpp::msg::RewardsResponse> {
            unreachable!()
        }
    }

    impl From<LppLocalStub> for Batch {
        fn from(_: LppLocalStub) -> Self {
            unreachable!()
        }
    }

    #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
    struct LppLocalStubUnreachable {}

    impl From<LppLocalStubUnreachable> for Batch {
        fn from(_: LppLocalStubUnreachable) -> Self {
            unreachable!()
        }
    }

    impl Lpp<TestCurrency> for LppLocalStubUnreachable {
        fn id(&self) -> Addr {
            unreachable!()
        }

        fn open_loan_req(&mut self, _amount: Coin<TestCurrency>) -> LppResult<()> {
            unreachable!()
        }

        fn open_loan_resp(&self, _resp: cosmwasm_std::Reply) -> LppResult<()> {
            unreachable!()
        }

        fn repay_loan_req(&mut self, _repayment: Coin<TestCurrency>) -> LppResult<()> {
            unreachable!()
        }

        fn loan(&self, _lease: impl Into<Addr>) -> LppResult<QueryLoanResponse<TestCurrency>> {
            unreachable!()
        }

        fn loan_outstanding_interest(
            &self,
            _lease: impl Into<Addr>,
            _by: Timestamp,
        ) -> LppResult<lpp::msg::QueryLoanOutstandingInterestResponse<TestCurrency>> {
            unreachable!()
        }

        fn quote(&self, _amount: Coin<TestCurrency>) -> LppResult<lpp::msg::QueryQuoteResponse> {
            unreachable!()
        }

        fn lpp_balance(&self) -> LppResult<lpp::msg::LppBalanceResponse<TestCurrency>> {
            unreachable!()
        }

        fn nlpn_price(&self) -> LppResult<lpp::msg::PriceResponse<TestCurrency>> {
            unreachable!()
        }

        fn config(&self) -> LppResult<lpp::msg::QueryConfigResponse> {
            unreachable!()
        }

        fn nlpn_balance(&self, _lender: impl Into<Addr>) -> LppResult<lpp::msg::BalanceResponse> {
            unreachable!()
        }

        fn rewards(&self, _lender: impl Into<Addr>) -> LppResult<lpp::msg::RewardsResponse> {
            unreachable!()
        }
    }

    fn create_lease<L>(lpp: L) -> Lease<TestCurrency, L>
    where
        L: Lpp<TestCurrency>,
    {
        let lpp_ref = LppRef::unchecked::<_, Nls>("lpp_adr");
        let loan_dto = LoanDTO::new(
            LEASE_START,
            lpp_ref,
            MARGIN_INTEREST_RATE,
            Duration::from_secs(100),
            Duration::from_secs(0),
        )
        .unwrap();
        Lease {
            customer: Addr::unchecked("customer"),
            currency: TestCurrency::SYMBOL.to_string(),
            liability: Liability::new(
                Percent::from_percent(65),
                Percent::from_percent(70),
                Percent::from_percent(80),
                10 * 24,
            ),
            loan: Loan::from_dto(loan_dto, lpp),
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
        lease
            .state(LEASE_STATE_AT, bank_account, Addr::unchecked("unused"))
            .unwrap()
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
            interest_rate,
            interest_rate_margin: MARGIN_INTEREST_RATE,
            principal_due: loan.principal_due,
            previous_margin_due: coin(0),
            previous_interest_due: coin(0),
            current_margin_due: coin(0),
            current_interest_due: coin(0),
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

        let res = lease
            .state(
                Timestamp::from_nanos(0),
                &bank_account,
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
