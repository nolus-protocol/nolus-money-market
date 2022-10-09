use std::marker::PhantomData;

use cosmwasm_std::{Addr, Timestamp};
use serde::Serialize;

use finance::{coin::Coin, currency::Currency, liability::Liability, price::Price};
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

pub(super) use self::{
    dto::LeaseDTO,
    liquidation::{LeaseInfo, LiquidationInfo, OnAlarmResult, Status, WarningLevel},
    repay::Result as RepayResult,
    with_lease::{execute, WithLease},
    with_lease_deps::{execute as execute_deps, WithLeaseDeps},
};

mod dto;
mod liquidation;
mod repay;
mod with_lease;
mod with_lease_deps;

pub struct Lease<'r, Lpn, Asset, Lpp, Profit, TimeAlarms, Oracle> {
    lease_addr: &'r Addr,
    customer: Addr,
    _asset: PhantomData<Asset>,
    liability: Liability,
    loan: Loan<Lpn, Lpp, Profit>,
    alarms: TimeAlarms,
    oracle: Oracle,
}

#[derive(Debug)]
pub(crate) struct IntoDTOResult {
    pub dto: LeaseDTO,
    pub batch: Batch,
}

impl<'r, Lpn, Asset, Lpp, Profit, TimeAlarms, Oracle>
    Lease<'r, Lpn, Asset, Lpp, Profit, TimeAlarms, Oracle>
where
    Lpn: Currency + Serialize,
    Asset: Currency + Serialize,
    Lpp: LppLenderTrait<Lpn>,
    TimeAlarms: TimeAlarmsTrait,
    Oracle: OracleTrait<Lpn>,
    Profit: ProfitTrait,
{
    pub(super) fn new(
        lease_addr: &'r Addr,
        customer: Addr,
        amount: Coin<Asset>,
        start_at: Timestamp,
        liability: Liability,
        loan: Loan<Lpn, Lpp, Profit>,
        deps: (TimeAlarms, Oracle),
    ) -> ContractResult<Self> {
        let mut res = Self {
            lease_addr,
            customer,
            _asset: PhantomData::<Asset>,
            liability,
            loan,
            alarms: deps.0,
            oracle: deps.1,
        };
        res.initial_alarm_schedule(amount, &start_at)?;
        Ok(res)
    }

    // TODO rename -> into_lease and move to the DTO
    pub(super) fn from_dto(
        dto: LeaseDTO,
        lease_addr: &'r Addr,
        lpp: Lpp,
        time_alarms: TimeAlarms,
        oracle: Oracle,
        profit: Profit,
    ) -> Self {
        Self {
            lease_addr,
            customer: dto.customer,
            liability: dto.liability,
            loan: Loan::from_dto(dto.loan, lpp, profit),
            alarms: time_alarms,
            oracle,
            _asset: PhantomData,
        }
    }

    pub(super) fn into_dto(self) -> IntoDTOResult {
        let (loan_dto, loan_batch) = self.loan.into_dto();

        let TimeAlarmsBatch {
            time_alarms_ref,
            batch: time_alarms_batch,
        } = self.alarms.into();

        let OracleBatch {
            oracle_ref,
            batch: oracle_batch,
        } = self.oracle.into();

        IntoDTOResult {
            dto: LeaseDTO::new(
                self.customer,
                ToOwned::to_owned(Asset::SYMBOL),
                self.liability,
                loan_dto,
                time_alarms_ref.into(),
                oracle_ref.into(),
            ),
            batch: loan_batch.merge(time_alarms_batch).merge(oracle_batch),
        }
    }

    pub(crate) fn owned_by(&self, addr: &Addr) -> bool {
        &self.customer == addr
    }

    pub(crate) fn sent_by_time_alarms(&self, addr: &Addr) -> bool {
        self.alarms.owned_by(addr)
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
        Ok(self.oracle.price_of::<Asset>()?)
    }
}

#[cfg(test)]
mod tests {

    use cosmwasm_std::{wasm_execute, Addr, Timestamp};
    use serde::{Deserialize, Serialize};

    use finance::{
        coin::Coin, currency::Currency, duration::Duration, interest::InterestPeriod,
        liability::Liability, percent::Percent, price::Price, test::currency::Usdc,
    };
    use lpp::{
        error::ContractError as LppError,
        msg::{LoanResponse, OutstandingInterest, QueryLoanResponse},
        stub::{
            lender::{LppLender, LppLenderRef},
            LppBatch,
        },
    };
    use market_price_oracle::msg::ExecuteMsg::AddPriceAlarm;
    use market_price_oracle::stub::{Oracle, OracleBatch, OracleRef};
    use marketprice::alarms::Alarm;
    use platform::{bank::BankAccountView, batch::Batch, error::Result as PlatformResult};
    use profit::stub::{Profit, ProfitBatch, ProfitRef};
    use time_alarms::{
        msg::ExecuteMsg::AddAlarm,
        stub::{TimeAlarms, TimeAlarmsBatch, TimeAlarmsRef},
    };

    use crate::{loan::Loan, msg::StateResponse, reply_id::ReplyId};

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

    type TestLpn = TestCurrency;

    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    pub struct LppLenderLocalStub {
        loan: Option<LoanResponse<TestLpn>>,
    }

    // TODO define a MockLpp trait to avoid implementing Lpp-s from scratch
    impl LppLender<TestLpn> for LppLenderLocalStub {
        fn open_loan_req(&mut self, _amount: Coin<TestLpn>) -> LppResult<()> {
            unreachable!()
        }

        fn open_loan_resp(&self, _resp: cosmwasm_std::Reply) -> LppResult<LoanResponse<TestLpn>> {
            unreachable!()
        }

        fn repay_loan_req(&mut self, _repayment: Coin<TestLpn>) -> LppResult<()> {
            Ok(())
        }

        fn loan(&self, _lease: impl Into<Addr>) -> LppResult<QueryLoanResponse<TestLpn>> {
            Ok(self.loan.clone())
        }

        fn loan_outstanding_interest(
            &self,
            _lease: impl Into<Addr>,
            by: Timestamp,
        ) -> LppResult<lpp::msg::QueryLoanOutstandingInterestResponse<TestLpn>> {
            Ok(self.loan.as_ref().map(|loan| {
                OutstandingInterest(
                    InterestPeriod::with_interest(loan.annual_interest_rate)
                        .spanning(Duration::between(loan.interest_paid, by))
                        .interest(loan.principal_due),
                )
            }))
        }

        fn quote(&self, _amount: Coin<TestLpn>) -> LppResult<lpp::msg::QueryQuoteResponse> {
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
            Self {
                lpp_ref: LppLenderRef::unchecked::<_, TestLpn>(
                    "local_test_lpp_lender_addr",
                    ReplyId::OpenLoanReq.into(),
                ),
                batch: Batch::default(),
            }
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
        pub(super) batch: Batch,
    }

    impl From<Addr> for TimeAlarmsLocalStub {
        fn from(alarms: Addr) -> Self {
            Self {
                address: alarms,
                batch: Batch::default(),
            }
        }
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

        fn price_of<C>(&self) -> market_price_oracle::stub::Result<Price<C, OracleBase>>
        where
            C: Currency,
        {
            Ok(Price::identity())
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

        fn price_of<C>(&self) -> market_price_oracle::stub::Result<Price<C, OracleBase>>
        where
            C: Currency,
        {
            Ok(Price::identity())
        }

        fn add_alarm(&mut self, _alarm: Alarm) -> market_price_oracle::stub::Result<()> {
            unreachable!()
        }
    }

    impl From<OracleLocalStubUnreachable> for OracleBatch {
        fn from(_: OracleLocalStubUnreachable) -> Self {
            Self {
                oracle_ref: OracleRef::unchecked::<_, TestLpn>(Addr::unchecked(
                    "local_test_oracle_addr",
                )),
                batch: Batch::default(),
            }
        }
    }

    pub struct ProfitLocalStub {
        address: Addr,
        pub batch: Batch,
    }

    impl Profit for ProfitLocalStub {
        fn send<C>(&mut self, _coins: Coin<C>)
        where
            C: Currency,
        {
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
        fn send<C>(&mut self, _coins: Coin<C>)
        where
            C: Currency,
        {
        }
    }

    impl From<ProfitLocalStubUnreachable> for ProfitBatch {
        fn from(_: ProfitLocalStubUnreachable) -> Self {
            Self {
                profit_ref: ProfitRef::unchecked(Addr::unchecked("local_test_profit_addr")),
                batch: Batch::default(),
            }
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
        let loan = Loan::new(
            LEASE_START,
            lpp,
            MARGIN_INTEREST_RATE,
            Duration::from_days(100),
            Duration::from_days(10),
            profit,
        )
        .unwrap();

        Lease::new(
            lease_addr,
            Addr::unchecked("customer"),
            0.into(),
            LEASE_START,
            Liability::new(
                Percent::from_percent(65),
                Percent::from_percent(5),
                Percent::from_percent(10),
                Percent::from_percent(2),
                Percent::from_percent(3),
                Percent::from_percent(2),
                24,
            ),
            loan,
            (time_alarms, oracle),
        )
        .unwrap()
    }

    pub fn load_lease<L, TA, O, P>(
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
        let into_dto = create_lease::<_, TimeAlarmsLocalStub, _, _>(
            lease_addr,
            LppLenderLocalStubUnreachable {},
            Addr::unchecked("dummy").into(),
            OracleLocalStubUnreachable {},
            ProfitLocalStubUnreachable {},
        )
        .into_dto();
        Lease::from_dto(into_dto.dto, lease_addr, lpp, time_alarms, oracle, profit)
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

        load_lease(
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
        let lease = load_lease(
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
