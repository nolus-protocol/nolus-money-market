use serde::Serialize;

use finance::{
    coin::Coin,
    currency::{self, Currency},
    liability::Liability,
    price::{total, Price},
};
use lpp::stub::lender::LppLender as LppLenderTrait;
use oracle::stub::{Oracle as OracleTrait, OracleBatch};
use platform::{bank::BankAccount, batch::Batch};
use profit::stub::Profit as ProfitTrait;
use sdk::cosmwasm_std::{Addr, Timestamp};
use timealarms::stub::{TimeAlarms as TimeAlarmsTrait, TimeAlarmsBatch};

use crate::{
    api::StateResponse,
    error::{ContractError, ContractResult},
    loan::Loan,
};

pub(super) use self::{
    dto::LeaseDTO,
    liquidation::{LeaseInfo, LiquidationInfo, OnAlarmResult, Status, WarningLevel},
    repay::Result as RepayResult,
};

mod dto;
mod liquidation;
mod repay;
pub(crate) mod with_lease;
pub(crate) mod with_lease_deps;

pub struct Lease<'r, Lpn, Asset, Lpp, Profit, TimeAlarms, Oracle>
where
    Asset: Currency,
{
    lease_addr: &'r Addr,
    customer: Addr,
    amount: Coin<Asset>,
    liability: Liability,
    loan: Loan<Lpn, Lpp, Profit>,
    alarms: TimeAlarms,
    oracle: Oracle,
}

#[cfg_attr(test, derive(Debug))]
pub struct IntoDTOResult {
    pub lease: LeaseDTO,
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
        debug_assert!(!amount.is_zero());

        let mut res = Self {
            lease_addr,
            customer,
            amount,
            liability,
            loan,
            alarms: deps.0,
            oracle: deps.1,
        };
        res.initial_alarm_schedule(&start_at)?;
        Ok(res)
    }

    pub(super) fn from_dto(
        dto: LeaseDTO,
        lease_addr: &'r Addr,
        lpp: Lpp,
        time_alarms: TimeAlarms,
        oracle: Oracle,
        profit: Profit,
    ) -> Self {
        let amount = dto.amount.try_into().expect(
            "The DTO -> Lease conversion should have resulted in Asset == dto.amount.symbol()",
        );
        Self {
            lease_addr,
            customer: dto.customer,
            amount,
            liability: dto.liability,
            loan: Loan::from_dto(dto.loan, lpp, profit),
            alarms: time_alarms,
            oracle,
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
            lease: LeaseDTO::new(
                self.customer,
                self.amount.into(),
                self.liability,
                loan_dto,
                time_alarms_ref,
                oracle_ref,
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

    pub(crate) fn close<B>(mut self, account: B) -> ContractResult<IntoDTOResult>
    where
        B: BankAccount,
    {
        let state = self.state(Timestamp::from_nanos(u64::MAX))?;
        match state {
            StateResponse::Opened { .. } => Err(ContractError::LoanNotPaid()),
            StateResponse::Paid(..) => {
                let bank_transfers = self.send_funds_to_customer(account)?;
                self.amount = Coin::<Asset>::default();

                let IntoDTOResult { lease, batch } = self.into_dto();
                Ok(IntoDTOResult {
                    lease,
                    batch: batch.merge(bank_transfers),
                })
            }
            StateResponse::Closed() => Err(ContractError::LoanClosed()),
        }
    }

    pub(crate) fn state(&self, now: Timestamp) -> ContractResult<StateResponse<Asset, Lpn>> {
        if self.amount.is_zero() {
            Ok(StateResponse::Closed())
        } else {
            let loan_state = self.loan.state(now, self.lease_addr.clone())?;

            loan_state.map_or(Ok(StateResponse::Paid(self.amount)), |state| {
                Ok(StateResponse::Opened {
                    amount: self.amount,
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

    fn lease_amount_lpn(&self) -> ContractResult<Coin<Lpn>> {
        Ok(total(self.amount, self.price_of_lease_currency()?))
    }

    fn send_funds_to_customer<B>(&self, mut account: B) -> ContractResult<Batch>
    where
        B: BankAccount,
    {
        let balance_lpn = account.balance::<Lpn>()?;
        let surplus = if currency::equal::<Lpn, Asset>() {
            // TODO remove once migrate to multi-currency version
            let lease_lpn = self.lease_amount_lpn()?;

            debug_assert!(balance_lpn >= lease_lpn);

            balance_lpn - lease_lpn
        } else {
            balance_lpn
        };

        if !surplus.is_zero() {
            account.send(surplus, &self.customer);
        }

        account.send(self.amount, &self.customer);

        Ok(account.into())
    }
}

#[cfg(test)]
mod tests {
    use serde::{Deserialize, Serialize};

    use finance::{
        coin::Coin,
        currency::{self, Currency},
        duration::Duration,
        interest::InterestPeriod,
        liability::Liability,
        percent::Percent,
        price::Price,
        test::currency::Usdc,
    };
    use lpp::{
        error::ContractError as LppError,
        msg::{LoanResponse, OutstandingInterest, QueryLoanResponse},
        stub::{
            lender::{LppLender, LppLenderRef},
            LppBatch,
        },
    };
    use marketprice::alarms::Alarm;
    use oracle::msg::ExecuteMsg::AddPriceAlarm;
    use oracle::stub::{Oracle, OracleBatch, OracleRef};
    use platform::{
        bank::{BankAccountView, BankStub},
        batch::Batch,
        error::Result as PlatformResult,
    };
    use profit::stub::{Profit, ProfitBatch, ProfitRef};
    use sdk::cosmwasm_std::{wasm_execute, Addr, BankMsg, Timestamp};
    use timealarms::{
        msg::ExecuteMsg::AddAlarm,
        stub::{TimeAlarms, TimeAlarmsBatch, TimeAlarmsRef},
    };

    use crate::{
        api::{InterestPaymentSpec, StateResponse},
        loan::Loan,
        reply_id::ReplyId,
    };

    use super::Lease;

    const CUSTOMER: &str = "customer";
    pub const MARGIN_INTEREST_RATE: Percent = Percent::from_permille(23);
    pub const LEASE_START: Timestamp = Timestamp::from_nanos(100);
    pub const LEASE_STATE_AT: Timestamp = Timestamp::from_nanos(200);
    pub type TestCurrency = Usdc;
    pub type LppResult<T> = Result<T, LppError>;

    type TestLpn = TestCurrency;

    pub struct MockBankView {
        balance: Coin<TestCurrency>,
    }

    impl BankAccountView for MockBankView {
        fn balance<C>(&self) -> PlatformResult<Coin<C>>
        where
            C: Currency,
        {
            assert!(currency::equal::<C, TestCurrency>());
            Ok(Coin::<C>::new(self.balance.into()))
        }
    }
    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    pub struct LppLenderLocalStub {
        loan: Option<LoanResponse<TestLpn>>,
    }

    impl From<Option<LoanResponse<TestLpn>>> for LppLenderLocalStub {
        fn from(loan: Option<LoanResponse<TestLpn>>) -> Self {
            Self { loan }
        }
    }

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
            Self {
                lpp_ref: LppLenderRef::unchecked::<_, TestLpn>(Addr::unchecked("test_lpp"), 0),
                batch: Batch::default(),
            }
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

        fn add_alarm(&mut self, time: Timestamp) -> timealarms::stub::Result<()> {
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

        fn add_alarm(&mut self, _time: Timestamp) -> timealarms::stub::Result<()> {
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

    impl From<Addr> for OracleLocalStub {
        fn from(oracle: Addr) -> Self {
            Self {
                address: oracle,
                batch: Batch::default(),
            }
        }
    }

    impl<OracleBase> Oracle<OracleBase> for OracleLocalStub
    where
        OracleBase: Currency + Serialize,
    {
        fn owned_by(&self, addr: &Addr) -> bool {
            &self.address == addr
        }

        fn price_of<C>(&self) -> oracle::stub::Result<Price<C, OracleBase>>
        where
            C: Currency,
        {
            Ok(Price::identity())
        }

        fn add_alarm(&mut self, alarm: Alarm) -> oracle::stub::Result<()> {
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

        fn price_of<C>(&self) -> oracle::stub::Result<Price<C, OracleBase>>
        where
            C: Currency,
        {
            Ok(Price::identity())
        }

        fn add_alarm(&mut self, _alarm: Alarm) -> oracle::stub::Result<()> {
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

    impl From<Addr> for ProfitLocalStub {
        fn from(profit: Addr) -> Self {
            Self {
                address: profit,
                batch: Batch::default(),
            }
        }
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
        amount: Coin<TestCurrency>,
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
            InterestPaymentSpec::new(Duration::from_days(100), Duration::from_days(10)),
            profit,
        );

        Lease::new(
            lease_addr,
            Addr::unchecked(CUSTOMER),
            amount,
            LEASE_START,
            Liability::new(
                Percent::from_percent(65),
                Percent::from_percent(5),
                Percent::from_percent(10),
                Percent::from_percent(2),
                Percent::from_percent(3),
                Percent::from_percent(2),
                Duration::from_hours(24),
            ),
            loan,
            (time_alarms, oracle),
        )
        .unwrap()
    }

    pub fn open_lease_with_deps<L, TA, O, P>(
        lease_addr: &Addr,
        amount: Coin<TestCurrency>,
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
            amount,
            LppLenderLocalStubUnreachable {},
            Addr::unchecked("dummy").into(),
            OracleLocalStubUnreachable {},
            ProfitLocalStubUnreachable {},
        )
        .into_dto();
        Lease::from_dto(into_dto.lease, lease_addr, lpp, time_alarms, oracle, profit)
    }

    pub fn open_lease(
        lease_addr: &Addr,
        amount: Coin<TestCurrency>,
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
        let lpp_stub: LppLenderLocalStub = loan_response.into();
        let time_alarms_stub: TimeAlarmsLocalStub = time_alarms_addr.into();
        let oracle_stub: OracleLocalStub = oracle_addr.into();
        let profit_stub: ProfitLocalStub = profit_addr.into();

        open_lease_with_deps(
            lease_addr,
            amount,
            lpp_stub,
            time_alarms_stub,
            oracle_stub,
            profit_stub,
        )
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
    ) -> StateResponse<TestCurrency, TestCurrency> {
        lease.state(LEASE_STATE_AT).unwrap()
    }

    pub fn coin(a: u128) -> Coin<TestCurrency> {
        Coin::<TestCurrency>::new(a)
    }

    #[test]
    // Open state -> Lease's balance in the loan's currency > 0, loan exists in the lpp
    fn state_opened() {
        let lease_amount = coin(1000);
        let interest_rate = Percent::from_permille(50);
        // LPP loan
        let loan = LoanResponse {
            principal_due: coin(300),
            interest_due: coin(0),
            annual_interest_rate: interest_rate,
            interest_paid: Timestamp::from_nanos(0),
        };

        let lease_addr = Addr::unchecked("lease");
        let lease = open_lease(
            &lease_addr,
            lease_amount,
            Some(loan.clone()),
            Addr::unchecked(String::new()),
            Addr::unchecked(String::new()),
            Addr::unchecked(String::new()),
        );

        let res = request_state(lease);
        let exp = StateResponse::Opened {
            amount: lease_amount,
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
        let lease_amount = coin(1000);
        let lease_addr = Addr::unchecked("lease");
        let lease = open_lease(
            &lease_addr,
            lease_amount,
            None,
            Addr::unchecked(String::new()),
            Addr::unchecked(String::new()),
            Addr::unchecked(String::new()),
        );

        let res = request_state(lease);
        let exp = StateResponse::Paid(lease_amount);
        assert_eq!(exp, res);
    }

    #[test]
    // Closed state -> Lease's balance in the loan's currency = 0, loan doesn't exist in the lpp anymore
    fn state_closed() {
        let lease_addr = Addr::unchecked("lease");
        let lease_amount = 10.into();
        let time_alarms_addr = Addr::unchecked(String::new());
        let oracle_addr = Addr::unchecked(String::new());
        let profit_addr = Addr::unchecked(String::new());
        let lease = open_lease(
            &lease_addr,
            lease_amount,
            None,
            time_alarms_addr,
            oracle_addr,
            profit_addr,
        );
        let account = BankStub::new(MockBankView {
            balance: lease_amount,
        });
        let res = lease.close(account).unwrap();
        let lease = Lease::<_, TestCurrency, _, _, _, _>::from_dto(
            res.lease,
            &lease_addr,
            LppLenderLocalStubUnreachable {},
            TimeAlarmsLocalStubUnreachable {},
            OracleLocalStubUnreachable {},
            ProfitLocalStubUnreachable {},
        );
        let res = lease.state(LEASE_STATE_AT).unwrap();
        let exp = StateResponse::Closed();
        assert_eq!(exp, res);
    }

    #[test]
    fn close_no_surplus() {
        let lease_addr = Addr::unchecked("lease");
        let lease_amount = 10.into();
        let time_alarms_addr = Addr::unchecked(String::new());
        let oracle_addr = Addr::unchecked(String::new());
        let profit_addr = Addr::unchecked(String::new());
        let lease = open_lease(
            &lease_addr,
            lease_amount,
            None,
            time_alarms_addr,
            oracle_addr,
            profit_addr,
        );
        let account = BankStub::new(MockBankView {
            balance: lease_amount,
        });
        let res = lease.close(account).unwrap();
        assert_eq!(res.batch, expect_bank_send(Batch::default(), lease_amount));
    }

    #[test]
    fn close_with_surplus() {
        let lease_addr = Addr::unchecked("lease");
        let lease_amount = 10.into();
        let surplus_amount = 2.into();
        let time_alarms_addr = Addr::unchecked(String::new());
        let oracle_addr = Addr::unchecked(String::new());
        let profit_addr = Addr::unchecked(String::new());
        let lease = open_lease(
            &lease_addr,
            lease_amount,
            None,
            time_alarms_addr,
            oracle_addr,
            profit_addr,
        );
        let account = BankStub::new(MockBankView {
            balance: lease_amount + surplus_amount,
        });
        let res = lease.close(account).unwrap();
        assert_eq!(res.batch, {
            let surplus_sent = expect_bank_send(Batch::default(), surplus_amount);
            expect_bank_send(surplus_sent, lease_amount)
        });
    }

    fn expect_bank_send(mut batch: Batch, amount: Coin<TestCurrency>) -> Batch {
        batch.schedule_execute_no_reply(BankMsg::Send {
            amount: vec![cosmwasm_std::coin(amount.into(), TestCurrency::BANK_SYMBOL)],
            to_address: CUSTOMER.into(),
        });
        batch
    }
}
