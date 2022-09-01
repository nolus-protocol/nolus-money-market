use std::marker::PhantomData;

use cosmwasm_std::{Addr, QuerierWrapper, Timestamp};
use serde::Serialize;

use finance::{
    currency::{Currency, SymbolOwned},
    liability::Liability,
};
use lpp::stub::Lpp as LppTrait;
use market_price_oracle::stub::Oracle as OracleTrait;
use platform::{
    bank::{BankAccount, BankAccountView},
    batch::Batch,
};

use crate::{
    error::{ContractError, ContractResult},
    loan::Loan,
    msg::StateResponse,
    oracle::Oracle as LeaseOracle,
};

pub(super) use self::{
    downpayment_dto::DownpaymentDTO,
    dto::LeaseDTO,
    liquidation::{LeaseInfo, OnAlarmResult, Status, WarningLevel},
    repay::Result as RepayResult,
};
use self::factory::Factory;

mod downpayment_dto;
mod dto;
mod factory;
mod liquidation;
mod open;
mod repay;

pub trait WithLease {
    type Output;
    type Error;

    fn exec<Lpn, Lpp, Oracle>(
        self,
        lease: Lease<Lpn, Lpp, Oracle>,
    ) -> Result<Self::Output, Self::Error>
    where
        Lpn: Currency + Serialize,
        Lpp: LppTrait<Lpn>,
        Oracle: OracleTrait<Lpn>;

    fn unknown_lpn(self, symbol: SymbolOwned) -> Result<Self::Output, Self::Error>;
}

pub fn execute<L, O, E>(dto: LeaseDTO, cmd: L, querier: &QuerierWrapper) -> Result<O, E>
where
    L: WithLease<Output = O, Error = E>,
{
    let lpp = dto.loan.lpp().clone();

    lpp.execute(Factory::new(cmd, dto, querier), querier)
}

pub struct Lease<Lpn, Lpp, Oracle> {
    customer: Addr,
    currency: SymbolOwned,
    liability: Liability,
    loan: Loan<Lpn, Lpp>,
    _oracle_c: PhantomData<Lpn>,
    oracle: LeaseOracle<Lpn, Oracle>,
}

impl<Lpn, Lpp, Oracle> Lease<Lpn, Lpp, Oracle>
where
    Lpn: Currency + Serialize,
    Lpp: LppTrait<Lpn>,
    Oracle: OracleTrait<Lpn>,
{
    pub(super) fn from_dto(dto: LeaseDTO, lpp: Lpp, oracle: Oracle) -> Self {
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
            _oracle_c: PhantomData,
            oracle: LeaseOracle::from_dto(dto.oracle, oracle),
        }
    }

    pub(super) fn into_dto(self) -> (LeaseDTO, Batch) {
        let (loan_dto, lpp_batch) = self.loan.into_dto();
        let (oracle_dto, oracle_batch) = self.oracle.into_dto();

        (
            LeaseDTO::new(
                self.customer,
                self.currency,
                self.liability,
                loan_dto,
                oracle_dto,
            ),
            lpp_batch.merge(oracle_batch),
        )
    }

    pub(crate) fn owned_by(&self, addr: &Addr) -> bool {
        &self.customer == addr
    }

    pub(crate) fn sent_by_oracle(&self, addr: &Addr) -> bool {
        self.oracle.owned_by(addr)
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

            loan_state.map_or(Ok(StateResponse::Paid(lease_amount)), |state| {
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
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use std::marker::PhantomData;

    use cosmwasm_std::{Addr, Timestamp, wasm_execute};
    use serde::{Deserialize, Serialize};

    use finance::{
        coin::Coin,
        currency::{Currency, Nls, Usdc},
        duration::Duration,
        interest::InterestPeriod,
        liability::Liability,
        percent::Percent,
    };
    use lpp::{
        error::ContractError as LppError,
        msg::{LoanResponse, OutstandingInterest, QueryLoanResponse},
        stub::{Lpp, LppRef, LppBatch},
    };
    use market_price_oracle::msg::ExecuteMsg::AddPriceAlarm;
    use market_price_oracle::msg::PriceResponse;
    use market_price_oracle::stub::{Oracle, OracleRef};
    use marketprice::alarms::Alarm;
    use marketprice::storage::Denom;
    use platform::{bank::BankAccountView, batch::Batch, error::Result as PlatformResult};

    use crate::{
        loan::{Loan, LoanDTO},
        msg::StateResponse,
        repay_id::ReplyId,
    };

    use super::{Lease, LeaseOracle};

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

        fn open_loan_resp(
            &self,
            _resp: cosmwasm_std::Reply,
        ) -> LppResult<LoanResponse<TestCurrency>> {
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
            by: Timestamp,
        ) -> LppResult<lpp::msg::QueryLoanOutstandingInterestResponse<TestCurrency>> {
            Ok(self.loan.as_ref().map(|loan| {
                OutstandingInterest(
                    InterestPeriod::with_interest(loan.annual_interest_rate)
                        .spanning(Duration::between(loan.interest_paid, by))
                        .interest(loan.principal_due),
                )
            }))
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

    impl From<LppLocalStub> for LppBatch {
        fn from(_: LppLocalStub) -> Self {
            unreachable!()
        }
    }

    #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
    struct LppLocalStubUnreachable {}

    impl From<LppLocalStubUnreachable> for LppBatch {
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

        fn open_loan_resp(
            &self,
            _resp: cosmwasm_std::Reply,
        ) -> LppResult<LoanResponse<TestCurrency>> {
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

    struct OracleLocalStub {
        address: Addr,
        batch: Batch,
    }

    impl From<OracleLocalStub> for Batch {
        fn from(stub: OracleLocalStub) -> Self {
            stub.batch
        }
    }

    impl<OracleBase> Oracle<OracleBase> for OracleLocalStub
    where
        OracleBase: Currency + Serialize,
    {
        fn get_price(&self, _denom: Denom) -> market_price_oracle::stub::Result<PriceResponse> {
            unimplemented!()
        }

        fn add_alarm(&mut self, alarm: Alarm) -> market_price_oracle::stub::Result<()> {
            self.batch.schedule_execute_no_reply(wasm_execute(
                self.address.clone(),
                &AddPriceAlarm { alarm },
                vec![],
            )?);

            Ok(())
        }
    }

    struct OracleLocalStubUnreachable;

    impl From<OracleLocalStubUnreachable> for Batch {
        fn from(_: OracleLocalStubUnreachable) -> Self {
            unreachable!()
        }
    }

    impl<OracleBase> Oracle<OracleBase> for OracleLocalStubUnreachable
    where
        OracleBase: Currency + Serialize,
    {
        fn get_price(&self, _denom: Denom) -> market_price_oracle::stub::Result<PriceResponse> {
            unreachable!()
        }

        fn add_alarm(&mut self, _alarm: Alarm) -> market_price_oracle::stub::Result<()> {
            unreachable!()
        }
    }

    fn create_lease<L, O>(lpp: L, oracle: O) -> Lease<TestCurrency, L, O>
    where
        L: Lpp<TestCurrency>,
        O: Oracle<TestCurrency>,
    {
        let lpp_ref = LppRef::unchecked::<_, Nls>("lpp_addr", Some(ReplyId::OpenLoanReq.into()));

        let loan_dto = LoanDTO::new(
            LEASE_START,
            lpp_ref,
            MARGIN_INTEREST_RATE,
            Duration::from_secs(100),
            Duration::from_secs(0),
        )
        .unwrap();

        let oracle_ref = OracleRef::unchecked::<_, Nls>("oracle_addr");

        Lease {
            customer: Addr::unchecked("customer"),
            currency: TestCurrency::SYMBOL.to_string(),
            liability: Liability::new(
                Percent::from_percent(65),
                Percent::from_percent(5),
                Percent::from_percent(10),
                Percent::from_percent(2),
                Percent::from_percent(3),
                Percent::from_percent(2),
                10 * 24,
            ),
            loan: Loan::from_dto(loan_dto, lpp),
            _oracle_c: PhantomData,
            oracle: LeaseOracle::from_dto(oracle_ref, oracle),
        }
    }

    fn lease_setup(
        loan_response: Option<LoanResponse<TestCurrency>>,
        oracle_addr: Addr,
    ) -> Lease<TestCurrency, LppLocalStub, OracleLocalStub> {
        let lpp_stub = LppLocalStub {
            loan: loan_response,
        };

        let oracle_stub = OracleLocalStub {
            address: oracle_addr,
            batch: Batch::default(),
        };

        create_lease(lpp_stub, oracle_stub)
    }

    fn create_bank_account(lease_amount: u128) -> BankStub {
        BankStub {
            balance: lease_amount,
        }
    }

    fn request_state(
        lease: Lease<TestCurrency, LppLocalStub, OracleLocalStub>,
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
        let lease = lease_setup(Some(loan.clone()), Addr::unchecked(String::new()));

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
        let lease = lease_setup(None, Addr::unchecked(String::new()));

        let res = request_state(lease, &bank_account);
        let exp = StateResponse::Paid(coin(lease_amount));
        assert_eq!(exp, res);
    }

    #[test]
    // Closed state -> Lease's balance in the loan's currency = 0, loan doesn't exist in the lpp anymore
    fn state_closed() {
        let lease_amount = 0;
        let bank_account = create_bank_account(lease_amount);
        let lease = lease_setup(None, Addr::unchecked(String::new()));

        let res = request_state(lease, &bank_account);
        let exp = StateResponse::Closed();
        assert_eq!(exp, res);
    }

    #[test]
    // Verify that if the Lease's balance is 0, lpp won't be queried for the loan
    fn state_closed_lpp_must_not_be_called() {
        let lpp_stub = LppLocalStubUnreachable {};
        let oracle_stub = OracleLocalStubUnreachable {};
        let lease = create_lease(lpp_stub, oracle_stub);

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
