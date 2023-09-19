use currency::{self, Currency};
use lpp::stub::loan::LppLoan as LppLoanTrait;
use oracle::stub::Oracle as OracleTrait;
use platform::batch::Batch;
use profit::stub::ProfitRef;
use sdk::cosmwasm_std::{Addr, Timestamp};
use timealarms::stub::TimeAlarmsRef;

use crate::{
    error::{ContractError, ContractResult},
    loan::Loan,
    position::{Position, PositionDTO},
};

pub(super) use self::{
    close::FullRepayReceipt, dto::LeaseDTO, paid::Lease as LeasePaid, state::State,
};

mod alarm;
mod close;
mod dto;
mod liquidation;
mod paid;
mod repay;
mod state;
pub(crate) mod with_lease;
pub(crate) mod with_lease_deps;
pub(crate) mod with_lease_paid;

// TODO look into reducing the type parameters to Lpn and Asset only!
// the others could be provided on demand when certain operation is being performed
// then review the methods that take `&mut self` whether could be transformed into `&self`
// and those that take `self` into `&mut self` or `&self`
pub struct Lease<Lpn, Asset, Lpp, Oracle> {
    addr: Addr,
    customer: Addr,
    position: Position<Asset, Lpn>,
    loan: Loan<Lpn, Lpp>,
    oracle: Oracle,
}

#[cfg_attr(test, derive(Debug))]
pub struct IntoDTOResult {
    pub lease: LeaseDTO,
    pub batch: Batch,
}

impl<Lpn, Asset, LppLoan, Oracle> Lease<Lpn, Asset, LppLoan, Oracle>
where
    Lpn: Currency,
    Asset: Currency,
    LppLoan: LppLoanTrait<Lpn>,
    Oracle: OracleTrait<Lpn>,
{
    pub(super) fn new(
        addr: Addr,
        customer: Addr,
        position: Position<Asset, Lpn>,
        loan: Loan<Lpn, LppLoan>,
        oracle: Oracle,
    ) -> Self {
        debug_assert!(!currency::equal::<Lpn, Asset>());
        // TODO specify that Lpn is of Lpns and Asset is of LeaseGroup

        Self {
            addr,
            customer,
            position,
            loan,
            oracle,
        }
    }

    pub(super) fn from_dto(dto: LeaseDTO, lpp_loan: LppLoan, oracle: Oracle) -> Self {
        let position = dto.position.try_into().expect("The DTO -> Lease conversion should have resulted in Asset == dto.position.amount.symbol() and Lpn == dto.position.min_asset.symbol()");
        Self::new(
            dto.addr,
            dto.customer,
            position,
            Loan::from_dto(dto.loan, lpp_loan),
            oracle,
        )
    }

    pub(crate) fn state(&self, now: Timestamp) -> State<Asset, Lpn> {
        let loan = self.loan.state(now);
        State {
            amount: self.position.amount(),
            interest_rate: loan.annual_interest,
            interest_rate_margin: loan.annual_interest_margin,
            principal_due: loan.principal_due,
            previous_margin_due: loan.previous_margin_interest_due,
            previous_interest_due: loan.previous_interest_due,
            current_margin_due: loan.current_margin_interest_due,
            current_interest_due: loan.current_interest_due,
            validity: now,
        }
    }
}

impl<Lpn, Asset, LppLoan, Oracle> Lease<Lpn, Asset, LppLoan, Oracle>
where
    Lpn: Currency,
    Asset: Currency,
    LppLoan: LppLoanTrait<Lpn>,
    LppLoan::Error: Into<ContractError>,
    Oracle: OracleTrait<Lpn>,
{
    pub(super) fn try_into_dto(
        self,
        profit: ProfitRef,
        time_alarms: TimeAlarmsRef,
    ) -> ContractResult<IntoDTOResult> {
        let (loan_dto, loan_batch) = self.loan.try_into_dto(profit)?;
        let position = PositionDTO::from(self.position);

        Ok(IntoDTOResult {
            lease: LeaseDTO::new(
                self.addr,
                self.customer,
                position,
                loan_dto,
                time_alarms,
                self.oracle.into(),
            ),
            batch: loan_batch,
        })
    }

    pub(super) fn try_into_messages(self) -> ContractResult<Batch> {
        self.loan.try_into_messages()
    }
}

#[cfg(test)]
mod tests {
    use serde::{Deserialize, Serialize};

    use ::currency::{lease::Atom, lpn::Usdc, Currency};
    use finance::{
        coin::Coin, duration::Duration, liability::Liability, percent::Percent, price::Price,
    };
    use lpp::{
        error::{ContractError as LppError, Result as LppResult},
        loan::RepayShares,
        msg::LoanResponse,
        stub::{loan::LppLoan, LppBatch, LppRef},
    };
    use oracle::stub::{Oracle, OracleRef};
    use platform::batch::Batch;
    use profit::stub::Profit;
    use sdk::cosmwasm_std::{Addr, Timestamp};

    use crate::{api::InterestPaymentSpec, loan::Loan, position::Position};

    use super::{Lease, State};

    const CUSTOMER: &str = "customer";
    const LEASE_ADDR: &str = "lease_addr";
    const ORACLE_ADDR: &str = "oracle_addr";
    const MARGIN_INTEREST_RATE: Percent = Percent::from_permille(23);
    pub(super) const LEASE_START: Timestamp = Timestamp::from_nanos(100);
    const LEASE_STATE_AT: Timestamp = Timestamp::from_nanos(200);
    const DUE_PERIOD: Duration = Duration::from_days(100);
    const GRACE_PERIOD: Duration = Duration::from_days(10);
    pub(super) const RECALC_TIME: Duration = Duration::from_hours(24);
    type TestLpn = Usdc;
    type TestCurrency = Atom;
    type TestLease = Lease<TestLpn, TestCurrency, LppLoanLocal<TestLpn>, OracleLocalStub>;

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

        fn repay(&mut self, by: Timestamp, repayment: Coin<Lpn>) -> RepayShares<Lpn> {
            self.loan.repay(by, repayment)
        }

        fn annual_interest_rate(&self) -> Percent {
            self.loan.annual_interest_rate
        }
    }

    impl<Lpn> TryFrom<LppLoanLocal<Lpn>> for LppBatch<LppRef>
    where
        Lpn: Currency,
    {
        type Error = LppError;

        fn try_from(_: LppLoanLocal<Lpn>) -> LppResult<Self> {
            Ok(Self {
                lpp_ref: LppRef::unchecked::<_, TestLpn>(Addr::unchecked("test_lpp")),
                batch: Batch::default(),
            })
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
        pub batch: Batch,
    }

    impl Profit for ProfitLocalStub {
        fn send<C>(&mut self, _coins: Coin<C>)
        where
            C: Currency,
        {
        }
    }

    impl From<ProfitLocalStub> for Batch {
        fn from(stub: ProfitLocalStub) -> Self {
            stub.batch
        }
    }

    pub fn open_lease(amount: Coin<TestCurrency>, loan: LoanResponse<TestLpn>) -> TestLease {
        open_lease_with_payment_spec(
            amount,
            loan,
            InterestPaymentSpec::new(DUE_PERIOD, GRACE_PERIOD),
        )
    }

    pub fn open_lease_with_payment_spec(
        amount: Coin<TestCurrency>,
        loan: LoanResponse<TestLpn>,
        interest_spec: InterestPaymentSpec,
    ) -> TestLease {
        let lease = Addr::unchecked(LEASE_ADDR);
        let oracle: OracleLocalStub = Addr::unchecked(ORACLE_ADDR).into();

        let loan = loan.into();
        let loan = Loan::new(LEASE_START, loan, MARGIN_INTEREST_RATE, interest_spec);
        Lease::new(
            lease,
            Addr::unchecked(CUSTOMER),
            Position::<TestCurrency, TestLpn>::new(
                amount,
                Liability::new(
                    Percent::from_percent(65),
                    Percent::from_percent(5),
                    Percent::from_percent(10),
                    Percent::from_percent(2),
                    Percent::from_percent(3),
                    Percent::from_percent(2),
                    RECALC_TIME,
                ),
                Coin::<TestLpn>::new(10_000),
                Coin::<TestLpn>::new(15_000_000),
            ),
            loan,
            oracle,
        )
    }

    pub fn request_state(
        lease: Lease<TestLpn, TestCurrency, LppLoanLocal<TestLpn>, OracleLocalStub>,
    ) -> State<TestCurrency, TestLpn> {
        lease.state(LEASE_STATE_AT)
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

        let lease = open_lease(lease_amount, loan.clone());

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
}
