use std::marker::PhantomData;

use cosmwasm_std::{Addr, QuerierWrapper, Timestamp};
use serde::Serialize;

use finance::{
    currency::{self, Currency, SymbolOwned},
    liability::Liability,
    price::Price,
};
use lpp::stub::lender::LppLender as LppLenderTrait;
use market_price_oracle::stub::{Oracle as OracleTrait, OracleBatch};
use platform::{
    bank::{BankAccount, BankAccountView},
    batch::Batch,
};
use profit::stub::Profit as ProfitTrait;
use time_alarms::stub::{TimeAlarms as TimeAlarmsTrait, TimeAlarmsBatch};

use crate::{
    error::{ContractError, ContractResult},
    loan::Loan,
    msg::StateResponse,
};

use self::factory::Factory;
pub(super) use self::{
    downpayment_dto::DownpaymentDTO,
    dto::LeaseDTO,
    liquidation::{LeaseInfo, LiquidationInfo, OnAlarmResult, Status, WarningLevel},
    repay::Result as RepayResult,
};

mod downpayment_dto;
mod dto;
mod factory;
mod liquidation;
mod open;
mod repay;

pub trait WithLease {
    type Output;
    type Error;

    fn exec<Lpn, Asset, Lpp, Profit, TimeAlarms, Oracle>(
        self,
        lease: Lease<Lpn, Asset, Lpp, Profit, TimeAlarms, Oracle>,
    ) -> Result<Self::Output, Self::Error>
    where
        Lpn: Currency + Serialize,
        Lpp: LppLenderTrait<Lpn>,
        TimeAlarms: TimeAlarmsTrait,
        Oracle: OracleTrait<Lpn>,
        Profit: ProfitTrait,
        Asset: Currency + Serialize;

    fn unknown_lpn(self, symbol: SymbolOwned) -> Result<Self::Output, Self::Error>;
}

pub fn execute<L, O, E>(
    dto: LeaseDTO,
    cmd: L,
    addr: &Addr,
    querier: &QuerierWrapper,
) -> Result<O, E>
where
    L: WithLease<Output = O, Error = E>,
{
    let lpp = dto.loan.lpp().clone();

    lpp.execute(Factory::new(cmd, dto, addr, querier), querier)
}

pub struct Lease<'r, Lpn, Asset, Lpp, Profit, TimeAlarms, Oracle> {
    lease_addr: &'r Addr,
    customer: Addr,
    liability: Liability,
    loan: Loan<Lpn, Lpp, Profit>,
    time_alarms: TimeAlarms,
    oracle: Oracle,
    _asset: PhantomData<Asset>,
}

impl<'r, Lpn, Asset, Lpp, Profit, TimeAlarms, Oracle>
    Lease<'r, Lpn, Asset, Lpp, Profit, TimeAlarms, Oracle>
where
    Lpn: Currency + Serialize,
    Lpp: LppLenderTrait<Lpn>,
    TimeAlarms: TimeAlarmsTrait,
    Oracle: OracleTrait<Lpn>,
    Profit: ProfitTrait,
    Asset: Currency + Serialize,
{
    pub(super) fn from_dto(
        dto: LeaseDTO,
        lease_addr: &'r Addr,
        lpp: Lpp,
        time_alarms: TimeAlarms,
        oracle: Oracle,
        profit: Profit,
    ) -> Self {
        assert_eq!(
            Lpn::SYMBOL,
            dto.currency,
            "[Single currency version] The LPN '{}' should match the currency of the lease '{}'",
            Lpn::SYMBOL,
            dto.currency
        );

        Self {
            lease_addr,
            customer: dto.customer,
            liability: dto.liability,
            loan: Loan::from_dto(dto.loan, lpp, profit),
            time_alarms,
            oracle,
            _asset: PhantomData,
        }
    }

    pub(super) fn into_dto(self) -> (LeaseDTO, Batch) {
        let (loan_dto, lpp_batch) = self.loan.into_dto();

        let TimeAlarmsBatch {
            time_alarms_ref: time_alarms_dto,
            batch: time_alarms_batch,
        } = self.time_alarms.into();

        let OracleBatch {
            oracle_ref: oracle_dto,
            batch: oracle_batch,
        } = self.oracle.into();

        (
            LeaseDTO::new(
                self.customer,
                ToOwned::to_owned(Asset::SYMBOL),
                self.liability,
                loan_dto,
                time_alarms_dto.into(),
                oracle_dto.into(),
            ),
            lpp_batch.merge(time_alarms_batch).merge(oracle_batch),
        )
    }

    pub(crate) fn owned_by(&self, addr: &Addr) -> bool {
        &self.customer == addr
    }

    pub(crate) fn sent_by_time_alarms(&self, addr: &Addr) -> bool {
        self.time_alarms.owned_by(addr)
    }

    pub(crate) fn sent_by_oracle(&self, addr: &Addr) -> bool {
        self.oracle.owned_by(addr)
    }

    // TODO add the lease address as a field in Lease<>
    //  and populate it on LeaseDTO.execute as LeaseFactory
    pub(crate) fn close<B>(self, mut account: B) -> ContractResult<Batch>
    where
        B: BankAccount,
    {
        let state = self.state(Timestamp::from_nanos(u64::MAX), &account)?;
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
    ) -> ContractResult<StateResponse<Lpn, Lpn>>
    where
        B: BankAccountView,
    {
        let lease_amount = account.balance::<Lpn>().map_err(ContractError::from)?;

        if lease_amount.is_zero() {
            Ok(StateResponse::Closed())
        } else {
            let loan_state = self.loan.state(now, self.lease_addr.clone())?;

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
                    validity: now,
                })
            })
        }
    }

    fn price_of_lease_currency(&self) -> ContractResult<Price<Asset, Lpn>> {
        if currency::equal::<Asset, Lpn>() {
            Ok(Price::identity())
        } else {
            self.oracle
                .price_of(ToOwned::to_owned(Asset::SYMBOL))?
                .price
                .try_into()
                .map_err(Into::into)
        }
    }
}

#[cfg(test)]
mod tests {
    use std::marker::PhantomData;

    use cosmwasm_std::{wasm_execute, Addr, Timestamp};
    use serde::{Deserialize, Serialize};

    use finance::{
        coin::Coin,
        currency::Currency,
        duration::Duration,
        interest::InterestPeriod,
        liability::Liability,
        percent::Percent,
        test::currency::{Nls, Usdc},
    };
    use lpp::{
        error::ContractError as LppError,
        msg::{LoanResponse, OutstandingInterest, QueryLoanResponse},
        stub::{
            lender::{LppLender, LppLenderRef},
            LppBatch,
        },
    };
    use market_price_oracle::{
        msg::ExecuteMsg::AddPriceAlarm,
        msg::PriceResponse,
        stub::{Oracle, OracleBatch, OracleRef},
    };
    use marketprice::{alarms::Alarm, storage::Denom};
    use platform::{bank::BankAccountView, batch::Batch, error::Result as PlatformResult};
    use profit::{
        error::Result as ProfitResult,
        stub::{Profit, ProfitBatch, ProfitRef},
    };
    use time_alarms::{
        msg::ExecuteMsg::AddAlarm,
        stub::{TimeAlarms, TimeAlarmsBatch, TimeAlarmsRef},
    };

    use crate::{
        loan::{Loan, LoanDTO},
        msg::StateResponse,
        repay_id::ReplyId,
    };

    use super::Lease;

    pub const MARGIN_INTEREST_RATE: Percent = Percent::from_permille(23);
    pub const LEASE_START: Timestamp = Timestamp::from_nanos(100);
    pub const LEASE_STATE_AT: Timestamp = Timestamp::from_nanos(200);
    pub type TestCurrency = Usdc;
    pub type LppResult<T> = Result<T, LppError>;

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
    pub struct LppLenderLocalStub {
        loan: Option<LoanResponse<TestCurrency>>,
    }

    // TODO define a MockLpp trait to avoid implementing Lpp-s from scratch
    impl LppLender<TestCurrency> for LppLenderLocalStub {
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
            Ok(())
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
    }

    impl From<LppLenderLocalStub> for LppBatch<LppLenderRef> {
        fn from(_: LppLenderLocalStub) -> Self {
            unreachable!()
        }
    }

    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    pub struct LppLenderLocalStubUnreachable {}

    impl From<LppLenderLocalStubUnreachable> for LppBatch<LppLenderRef> {
        fn from(_: LppLenderLocalStubUnreachable) -> Self {
            unreachable!()
        }
    }

    impl LppLender<TestCurrency> for LppLenderLocalStubUnreachable {
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
    }

    pub struct TimeAlarmsLocalStub {
        address: Addr,
        pub batch: Batch,
    }

    impl TimeAlarms for TimeAlarmsLocalStub {
        fn owned_by(&self, addr: &Addr) -> bool {
            &self.address == addr
        }

        fn add_alarm(&mut self, time: Timestamp) -> time_alarms::stub::Result<()> {
            self.batch.schedule_execute_no_reply(wasm_execute(
                self.address.clone(),
                &AddAlarm { time },
                vec![],
            )?);

            Ok(())
        }
    }

    impl From<TimeAlarmsLocalStub> for TimeAlarmsBatch {
        fn from(stub: TimeAlarmsLocalStub) -> Self {
            TimeAlarmsBatch {
                time_alarms_ref: TimeAlarmsRef::unchecked(stub.address),
                batch: stub.batch,
            }
        }
    }

    pub struct TimeAlarmsLocalStubUnreachable;

    impl TimeAlarms for TimeAlarmsLocalStubUnreachable {
        fn owned_by(&self, _addr: &Addr) -> bool {
            unreachable!()
        }

        fn add_alarm(&mut self, _time: Timestamp) -> time_alarms::stub::Result<()> {
            unreachable!()
        }
    }

    impl From<TimeAlarmsLocalStubUnreachable> for TimeAlarmsBatch {
        fn from(_: TimeAlarmsLocalStubUnreachable) -> Self {
            unreachable!()
        }
    }

    pub struct OracleLocalStub {
        address: Addr,
        pub batch: Batch,
    }

    impl<OracleBase> Oracle<OracleBase> for OracleLocalStub
    where
        OracleBase: Currency + Serialize,
    {
        fn owned_by(&self, addr: &Addr) -> bool {
            &self.address == addr
        }

        fn price_of(&self, _denom: Denom) -> market_price_oracle::stub::Result<PriceResponse> {
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

    impl From<OracleLocalStub> for OracleBatch {
        fn from(stub: OracleLocalStub) -> Self {
            OracleBatch {
                oracle_ref: OracleRef::unchecked::<_, TestCurrency>(stub.address),
                batch: stub.batch,
            }
        }
    }

    pub struct OracleLocalStubUnreachable;

    impl<OracleBase> Oracle<OracleBase> for OracleLocalStubUnreachable
    where
        OracleBase: Currency + Serialize,
    {
        fn owned_by(&self, _addr: &Addr) -> bool {
            unreachable!()
        }

        fn price_of(&self, _denom: Denom) -> market_price_oracle::stub::Result<PriceResponse> {
            unreachable!()
        }

        fn add_alarm(&mut self, _alarm: Alarm) -> market_price_oracle::stub::Result<()> {
            unreachable!()
        }
    }

    impl From<OracleLocalStubUnreachable> for OracleBatch {
        fn from(_: OracleLocalStubUnreachable) -> Self {
            unreachable!()
        }
    }

    pub struct ProfitLocalStub {
        address: Addr,
        pub batch: Batch,
    }

    impl Profit for ProfitLocalStub {
        fn send<C>(&mut self, _coins: Coin<C>) -> ProfitResult<()>
        where
            C: Currency,
        {
            Ok(())
        }
    }

    impl From<ProfitLocalStub> for ProfitBatch {
        fn from(stub: ProfitLocalStub) -> Self {
            ProfitBatch {
                profit_ref: ProfitRef::unchecked(stub.address),
                batch: stub.batch,
            }
        }
    }

    pub struct ProfitLocalStubUnreachable;

    impl Profit for ProfitLocalStubUnreachable {
        fn send<C>(&mut self, _coins: Coin<C>) -> ProfitResult<()>
        where
            C: Currency,
        {
            Ok(())
        }
    }

    impl From<ProfitLocalStubUnreachable> for ProfitBatch {
        fn from(_: ProfitLocalStubUnreachable) -> Self {
            unreachable!()
        }
    }

    pub fn create_lease<L, TA, O, P>(
        lease_addr: &Addr,
        lpp: L,
        time_alarms: TA,
        oracle: O,
        profit: P,
    ) -> Lease<TestCurrency, TestCurrency, L, P, TA, O>
    where
        L: LppLender<TestCurrency>,
        TA: TimeAlarms,
        O: Oracle<TestCurrency>,
        P: Profit,
    {
        let lpp_ref = LppLenderRef::unchecked::<_, Nls>("lpp_addr", ReplyId::OpenLoanReq.into());

        let profit_ref = ProfitRef::unchecked("profit_addr");

        let loan_dto = LoanDTO::new(
            LEASE_START,
            lpp_ref,
            MARGIN_INTEREST_RATE,
            Duration::from_days(100),
            Duration::from_days(10),
            profit_ref,
        )
        .unwrap();

        Lease {
            lease_addr,
            customer: Addr::unchecked("customer"),
            liability: Liability::new(
                Percent::from_percent(65),
                Percent::from_percent(5),
                Percent::from_percent(10),
                Percent::from_percent(2),
                Percent::from_percent(3),
                Percent::from_percent(2),
                24,
            ),
            loan: Loan::from_dto(loan_dto, lpp, profit),
            time_alarms,
            oracle,
            _asset: PhantomData,
        }
    }

    pub fn lease_setup(
        lease_addr: &Addr,
        loan_response: Option<LoanResponse<TestCurrency>>,
        time_alarms_addr: Addr,
        oracle_addr: Addr,
        profit_addr: Addr,
    ) -> Lease<
        TestCurrency,
        TestCurrency,
        LppLenderLocalStub,
        ProfitLocalStub,
        TimeAlarmsLocalStub,
        OracleLocalStub,
    > {
        let lpp_stub = LppLenderLocalStub {
            loan: loan_response,
        };

        let time_alarms_stub = TimeAlarmsLocalStub {
            address: time_alarms_addr,
            batch: Batch::default(),
        };

        let oracle_stub = OracleLocalStub {
            address: oracle_addr,
            batch: Batch::default(),
        };

        let profit_stub = ProfitLocalStub {
            address: profit_addr,
            batch: Batch::default(),
        };

        create_lease(
            lease_addr,
            lpp_stub,
            time_alarms_stub,
            oracle_stub,
            profit_stub,
        )
    }

    pub fn create_bank_account(lease_amount: u128) -> BankStub {
        BankStub {
            balance: lease_amount,
        }
    }

    pub fn request_state(
        lease: Lease<
            TestCurrency,
            TestCurrency,
            LppLenderLocalStub,
            ProfitLocalStub,
            TimeAlarmsLocalStub,
            OracleLocalStub,
        >,
        bank_account: &BankStub,
    ) -> StateResponse<TestCurrency, TestCurrency> {
        lease.state(LEASE_STATE_AT, bank_account).unwrap()
    }

    pub fn coin(a: u128) -> Coin<TestCurrency> {
        Coin::<TestCurrency>::new(a)
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
        let lease_addr = Addr::unchecked("lease");
        let lease = lease_setup(
            &lease_addr,
            Some(loan.clone()),
            Addr::unchecked(String::new()),
            Addr::unchecked(String::new()),
            Addr::unchecked(String::new()),
        );

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
            validity: LEASE_STATE_AT,
        };

        assert_eq!(exp, res);
    }

    #[test]
    // Paid state -> Lease's balance in the loan's currency > 0, loan doesn't exist in the lpp anymore
    fn state_paid() {
        let lease_amount = 1000;
        let bank_account = create_bank_account(lease_amount);
        let lease_addr = Addr::unchecked("lease");
        let lease = lease_setup(
            &lease_addr,
            None,
            Addr::unchecked(String::new()),
            Addr::unchecked(String::new()),
            Addr::unchecked(String::new()),
        );

        let res = request_state(lease, &bank_account);
        let exp = StateResponse::Paid(coin(lease_amount));
        assert_eq!(exp, res);
    }

    #[test]
    // Closed state -> Lease's balance in the loan's currency = 0, loan doesn't exist in the lpp anymore
    fn state_closed() {
        let lease_amount = 0;
        let bank_account = create_bank_account(lease_amount);
        let lease_addr = Addr::unchecked("lease");
        let lease = lease_setup(
            &lease_addr,
            None,
            Addr::unchecked(String::new()),
            Addr::unchecked(String::new()),
            Addr::unchecked(String::new()),
        );

        let res = request_state(lease, &bank_account);
        let exp = StateResponse::Closed();
        assert_eq!(exp, res);
    }

    #[test]
    // Verify that if the Lease's balance is 0, lpp won't be queried for the loan
    fn state_closed_lpp_must_not_be_called() {
        let lpp_stub = LppLenderLocalStubUnreachable {};
        let time_alarms_stub = TimeAlarmsLocalStubUnreachable {};
        let oracle_stub = OracleLocalStubUnreachable {};
        let profit_stub = ProfitLocalStubUnreachable {};
        let lease_addr = Addr::unchecked("lease");
        let lease = create_lease(
            &lease_addr,
            lpp_stub,
            time_alarms_stub,
            oracle_stub,
            profit_stub,
        );

        let bank_account = create_bank_account(0);

        let res = lease
            .state(Timestamp::from_nanos(0), &bank_account)
            .unwrap();

        let exp = StateResponse::Closed();
        assert_eq!(exp, res);
    }
}
