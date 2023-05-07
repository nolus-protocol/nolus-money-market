use serde::Serialize;

use finance::{
    coin::Coin,
    currency::{self, Currency},
    liability::Liability,
    price::Price,
};
use lpp::stub::loan::LppLoan as LppLoanTrait;
use oracle::stub::Oracle as OracleTrait;
use platform::{bank::BankAccount, batch::Batch};
use profit::stub::Profit as ProfitTrait;
use sdk::cosmwasm_std::{Addr, Timestamp};
use timealarms::stub::TimeAlarmsRef;

use crate::{error::ContractResult, loan::Loan};

pub(super) use self::{
    dto::LeaseDTO,
    liquidation::{Cause, Liquidation, Status},
    state::State,
};

mod alarm;
mod dto;
mod liquidation;
mod repay;
mod state;
pub(crate) mod with_lease;
pub(crate) mod with_lease_deps;

// TODO look into reducing the type parameters to Lpn and Asset only!
// the others could be provided on demand when certain operation is being performed
// then review the methods that take `&mut self` whether could be transformed into `&self`
// and those that take `self` into `&mut self` or `&self`
pub struct Lease<Lpn, Asset, Lpp, Profit, Oracle> {
    addr: Addr,
    customer: Addr,
    amount: Coin<Asset>,
    liability: Liability,
    loan: Loan<Lpn, Lpp, Profit>,
    oracle: Oracle,
}

#[cfg_attr(test, derive(Debug))]
pub struct IntoDTOResult {
    pub lease: LeaseDTO,
    pub batch: Batch,
}

impl<Lpn, Asset, LppLoan, Profit, Oracle> Lease<Lpn, Asset, LppLoan, Profit, Oracle>
where
    Lpn: Currency + Serialize,
    Asset: Currency + Serialize,
    LppLoan: LppLoanTrait<Lpn>,
    Oracle: OracleTrait<Lpn>,
    Profit: ProfitTrait,
{
    pub(super) fn new(
        addr: Addr,
        customer: Addr,
        amount: Coin<Asset>,
        liability: Liability,
        loan: Loan<Lpn, LppLoan, Profit>,
        oracle: Oracle,
    ) -> Self {
        debug_assert!(!amount.is_zero());
        debug_assert!(!currency::equal::<Lpn, Asset>());
        // TODO specify that Lpn is of Lpns and Asset is of LeaseGroup

        Self {
            addr,
            customer,
            amount,
            liability,
            loan,
            oracle,
        }
    }

    pub(super) fn from_dto(
        dto: LeaseDTO,
        lpp_loan: Option<LppLoan>,
        oracle: Oracle,
        profit: Profit,
    ) -> Self {
        let amount = dto.amount.try_into().expect(
            "The DTO -> Lease conversion should have resulted in Asset == dto.amount.symbol()",
        );
        Self {
            addr: dto.addr,
            customer: dto.customer,
            amount,
            liability: dto.liability,
            loan: Loan::from_dto(dto.loan, lpp_loan, profit),
            oracle,
        }
    }

    pub(super) fn into_dto(self, time_alarms: TimeAlarmsRef) -> IntoDTOResult {
        let (loan_dto, loan_batch) = self.loan.into_dto();

        IntoDTOResult {
            lease: LeaseDTO::new(
                self.addr,
                self.customer,
                self.amount.into(),
                self.liability,
                loan_dto,
                time_alarms,
                self.oracle.into(),
            ),
            batch: loan_batch,
        }
    }

    //TODO take this out into a dedicated type `LeasePaid`
    pub(crate) fn close<B>(self, lease_account: B) -> ContractResult<Batch>
    where
        B: BankAccount,
    {
        debug_assert!(self
            .loan
            .state(Timestamp::from_nanos(u64::MAX), self.addr.clone())?
            .is_none());

        self.send_funds_to_customer(lease_account)
    }

    pub(crate) fn state(&self, now: Timestamp) -> ContractResult<State<Asset, Lpn>> {
        self.loan.state(now, self.addr.clone()).map(|loan_state| {
            let loan = loan_state.expect("not paid");
            State {
                amount: self.amount,
                interest_rate: loan.annual_interest,
                interest_rate_margin: loan.annual_interest_margin,
                principal_due: loan.principal_due,
                previous_margin_due: loan.previous_margin_interest_due,
                previous_interest_due: loan.previous_interest_due,
                current_margin_due: loan.current_margin_interest_due,
                current_interest_due: loan.current_interest_due,
                validity: now,
            }
        })
    }

    fn price_of_lease_currency(&self) -> ContractResult<Price<Asset, Lpn>> {
        Ok(self.oracle.price_of::<Asset>()?)
    }

    fn send_funds_to_customer<B>(&self, mut lease_account: B) -> ContractResult<Batch>
    where
        B: BankAccount,
    {
        let surplus = lease_account.balance::<Lpn>()?;

        if !surplus.is_zero() {
            lease_account.send(surplus, &self.customer);
        }

        lease_account.send(self.amount, &self.customer);

        Ok(lease_account.into())
    }
}

#[cfg(test)]
mod tests {
    use serde::{Deserialize, Serialize};

    use ::currency::{lease::Atom, lpn::Usdc};
    use finance::{
        coin::{Coin, WithCoin},
        currency::{self, Currency, Group},
        duration::Duration,
        liability::Liability,
        percent::Percent,
        price::Price,
        zero::Zero,
    };
    use lpp::{
        msg::LoanResponse,
        stub::{loan::LppLoan, LppBatch, LppRef},
    };
    use oracle::stub::{Oracle, OracleRef};
    use platform::{
        bank::{Aggregate, BalancesResult, BankAccountView, BankStub},
        batch::Batch,
        error::Result as PlatformResult,
    };
    use profit::stub::{Profit, ProfitBatch, ProfitRef};
    use sdk::cosmwasm_std::{Addr, BankMsg, Timestamp};

    use crate::{api::InterestPaymentSpec, loan::Loan};

    use super::{Lease, State};

    const CUSTOMER: &str = "customer";
    pub const MARGIN_INTEREST_RATE: Percent = Percent::from_permille(23);
    pub const LEASE_START: Timestamp = Timestamp::from_nanos(100);
    pub const LEASE_STATE_AT: Timestamp = Timestamp::from_nanos(200);
    type TestLpn = Usdc;
    pub type TestCurrency = Atom;

    pub fn loan<Lpn>() -> LoanResponse<Lpn>
    where
        Lpn: Currency,
    {
        LoanResponse {
            principal_due: Coin::from(100),
            annual_interest_rate: Percent::from_percent(10),
            interest_paid: LEASE_START,
        }
    }
    pub struct MockBankView {
        balance: Coin<TestCurrency>,
        balance_surplus: Coin<TestLpn>,
    }

    impl MockBankView {
        fn new(amount: Coin<TestCurrency>, amount_surplus: Coin<TestLpn>) -> Self {
            Self {
                balance: amount,
                balance_surplus: amount_surplus,
            }
        }
        fn only_balance(amount: Coin<TestCurrency>) -> Self {
            Self {
                balance: amount,
                balance_surplus: Coin::ZERO,
            }
        }
    }

    impl BankAccountView for MockBankView {
        fn balance<C>(&self) -> PlatformResult<Coin<C>>
        where
            C: Currency,
        {
            if currency::equal::<C, TestCurrency>() {
                Ok(Coin::<C>::new(self.balance.into()))
            } else if currency::equal::<C, TestLpn>() {
                Ok(Coin::<C>::new(self.balance_surplus.into()))
            } else {
                unreachable!("Expected {}, found {}", TestCurrency::TICKER, C::TICKER);
            }
        }

        fn balances<G, Cmd>(&self, _: Cmd) -> BalancesResult<Cmd>
        where
            G: Group,
            Cmd: WithCoin,
            Cmd::Output: Aggregate,
        {
            unimplemented!()
        }
    }
    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    pub struct LppLoanLocal<Lpn>
    where
        Lpn: Currency,
    {
        loan: LoanResponse<Lpn>,
    }

    impl<Lpn> From<LoanResponse<Lpn>> for LppLoanLocal<Lpn>
    where
        Lpn: Currency,
    {
        fn from(loan: LoanResponse<Lpn>) -> Self {
            Self { loan }
        }
    }

    impl<Lpn> LppLoan<Lpn> for LppLoanLocal<Lpn>
    where
        Lpn: Currency,
    {
        fn principal_due(&self) -> Coin<Lpn> {
            self.loan.principal_due
        }

        fn interest_due(&self, by: Timestamp) -> Coin<Lpn> {
            self.loan.interest_due(by)
        }

        fn repay(&mut self, _repayment: Coin<Lpn>) -> lpp::error::Result<()> {
            todo!()
        }

        fn annual_interest_rate(&self) -> Percent {
            self.loan.annual_interest_rate
        }
    }

    impl<Lpn> From<LppLoanLocal<Lpn>> for LppBatch<LppRef>
    where
        Lpn: Currency,
    {
        fn from(_: LppLoanLocal<Lpn>) -> Self {
            Self {
                lpp_ref: LppRef::unchecked::<_, TestLpn>(Addr::unchecked("test_lpp")),
                batch: Batch::default(),
            }
        }
    }

    pub struct OracleLocalStub {
        address: Addr,
    }

    impl From<Addr> for OracleLocalStub {
        fn from(oracle: Addr) -> Self {
            Self { address: oracle }
        }
    }

    impl<OracleBase> Oracle<OracleBase> for OracleLocalStub
    where
        Self: Into<OracleRef>,
        OracleBase: Currency + Serialize,
    {
        fn price_of<C>(&self) -> oracle::stub::Result<Price<C, OracleBase>>
        where
            C: Currency,
        {
            Ok(Price::identity())
        }
    }

    impl From<OracleLocalStub> for OracleRef {
        fn from(stub: OracleLocalStub) -> Self {
            OracleRef::unchecked::<_, TestCurrency>(stub.address)
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

    pub fn create_lease<Lpn, AssetC, L, O, P>(
        addr: Addr,
        amount: Coin<AssetC>,
        lpp: Option<L>,
        oracle: O,
        profit: P,
    ) -> Lease<Lpn, AssetC, L, P, O>
    where
        Lpn: Currency + Serialize,
        AssetC: Currency + Serialize,
        L: LppLoan<Lpn>,
        O: Oracle<Lpn>,
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
            addr,
            Addr::unchecked(CUSTOMER),
            amount,
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
            oracle,
        )
    }

    pub fn open_lease(
        lease_addr: Addr,
        amount: Coin<TestCurrency>,
        loan_response: Option<LoanResponse<TestLpn>>,
        oracle_addr: Addr,
        profit_addr: Addr,
    ) -> Lease<TestLpn, TestCurrency, LppLoanLocal<TestLpn>, ProfitLocalStub, OracleLocalStub> {
        let lpp = loan_response.map(Into::into);
        let oracle: OracleLocalStub = oracle_addr.into();
        let profit: ProfitLocalStub = profit_addr.into();

        create_lease::<TestLpn, TestCurrency, _, _, _>(lease_addr, amount, lpp, oracle, profit)
    }

    pub fn request_state(
        lease: Lease<
            TestLpn,
            TestCurrency,
            LppLoanLocal<TestLpn>,
            ProfitLocalStub,
            OracleLocalStub,
        >,
    ) -> State<TestCurrency, TestLpn> {
        lease.state(LEASE_STATE_AT).unwrap()
    }

    pub fn coin(a: u128) -> Coin<TestCurrency> {
        Coin::new(a)
    }

    pub fn lpn_coin(a: u128) -> Coin<TestLpn> {
        Coin::new(a)
    }

    #[test]
    // Open state -> Lease's balance in the loan's currency > 0, loan exists in the lpp
    fn state_opened() {
        let lease_amount = coin(1000);
        let interest_rate = Percent::from_permille(50);
        // LPP loan
        let loan = LoanResponse {
            principal_due: lpn_coin(300),
            annual_interest_rate: interest_rate,
            interest_paid: Timestamp::from_nanos(0),
        };

        let lease_addr = Addr::unchecked("lease");
        let lease = open_lease(
            lease_addr,
            lease_amount,
            Some(loan.clone()),
            Addr::unchecked(String::new()),
            Addr::unchecked(String::new()),
        );

        let res = request_state(lease);
        let exp = State {
            amount: lease_amount,
            interest_rate,
            interest_rate_margin: MARGIN_INTEREST_RATE,
            principal_due: loan.principal_due,
            previous_margin_due: lpn_coin(0),
            previous_interest_due: lpn_coin(0),
            current_margin_due: lpn_coin(0),
            current_interest_due: lpn_coin(0),
            validity: LEASE_STATE_AT,
        };

        assert_eq!(exp, res);
    }

    #[test]
    fn close_no_surplus() {
        let lease_addr = Addr::unchecked("lease");
        let lease_amount = 10.into();
        let oracle_addr = Addr::unchecked(String::new());
        let profit_addr = Addr::unchecked(String::new());
        let lease = open_lease(lease_addr, lease_amount, None, oracle_addr, profit_addr);
        let lease_account = BankStub::new(MockBankView::only_balance(lease_amount));
        let res = lease.close(lease_account).unwrap();
        assert_eq!(res, expect_bank_send(Batch::default(), lease_amount));
    }

    #[test]
    fn close_with_surplus() {
        let lease_addr = Addr::unchecked("lease");
        let lease_amount = 10.into();
        let surplus_amount = 2.into();
        let oracle_addr = Addr::unchecked(String::new());
        let profit_addr = Addr::unchecked(String::new());
        let lease = open_lease(lease_addr, lease_amount, None, oracle_addr, profit_addr);
        let lease_account = BankStub::new(MockBankView::new(lease_amount, surplus_amount));
        let res = lease.close(lease_account).unwrap();
        assert_eq!(res, {
            let surplus_sent = expect_bank_send(Batch::default(), surplus_amount);
            expect_bank_send(surplus_sent, lease_amount)
        });
    }

    fn expect_bank_send<C>(mut batch: Batch, amount: Coin<C>) -> Batch
    where
        C: Currency,
    {
        batch.schedule_execute_no_reply(BankMsg::Send {
            amount: vec![cosmwasm_std::coin(amount.into(), C::BANK_SYMBOL)],
            to_address: CUSTOMER.into(),
        });
        batch
    }
}
