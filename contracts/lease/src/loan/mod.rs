mod state;
mod repay;

use platform::batch::Batch;
pub use state::State;

use std::{fmt::Debug, marker::PhantomData};

use cosmwasm_std::{Addr, Reply, Timestamp};
use finance::{
    coin::Coin,
    currency::Currency,
    duration::Duration,
    interest::InterestPeriod,
    percent::{Percent, Units},
};
use lpp::{
    msg::{LoanResponse, QueryLoanResponse},
    stub::{Lpp as LppTrait, LppRef},
};
use serde::{Deserialize, Serialize};

use crate::{
    error::ContractError,
    error::ContractResult,
};

pub(crate) use repay::{Result as RepayResult, Receipt};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub(crate) struct LoanDTO {
    annual_margin_interest: Percent,
    lpp: LppRef,
    interest_due_period: Duration,
    grace_period: Duration,
    current_period: InterestPeriod<Units, Percent>,
}

impl LoanDTO {
    pub(crate) fn new(
        start: Timestamp,
        lpp: LppRef,
        annual_margin_interest: Percent,
        interest_due_period: Duration,
        grace_period: Duration,
    ) -> Self {
        Self {
            annual_margin_interest,
            lpp,
            interest_due_period,
            grace_period,
            current_period: InterestPeriod::with_interest(annual_margin_interest)
                .from(start)
                .spanning(interest_due_period),
        }
    }

    pub(super) fn lpp(&self) -> &LppRef {
        &self.lpp
    }
}

pub struct Loan<Lpn, Lpp> {
    annual_margin_interest: Percent,
    lpn: PhantomData<Lpn>,
    lpp: Lpp,
    interest_due_period: Duration,
    _grace_period: Duration,
    current_period: InterestPeriod<Units, Percent>,
}

impl<Lpn, Lpp> Loan<Lpn, Lpp>
    where
        Lpp: LppTrait<Lpn>,
        Lpn: Currency + Debug,
{
    pub(super) fn from_dto(dto: LoanDTO, lpp: Lpp) -> Self {
        Self {
            annual_margin_interest: dto.annual_margin_interest,
            lpn: PhantomData,
            lpp,
            interest_due_period: dto.interest_due_period,
            _grace_period: dto.grace_period,
            current_period: dto.current_period,
        }
    }

    pub(crate) fn open_loan_req(mut self, amount: Coin<Lpn>) -> ContractResult<Batch> {
        self.lpp.open_loan_req(amount)?;
        Ok(self.into())
    }

    pub(crate) fn open_loan_resp(self, resp: Reply) -> ContractResult<Batch> {
        self.lpp.open_loan_resp(resp)?;
        Ok(self.into())
    }

    pub(crate) fn repay(
        mut self,
        payment: Coin<Lpn>,
        by: Timestamp,
        lease: Addr,
    ) -> ContractResult<RepayResult<Lpn>> {
        self.repay_inner(payment, by, lease)
            .map(|paid| RepayResult {
                batch: self.into(),
                receipt: paid,
            })
    }

    fn repay_inner(
        &mut self,
        payment: Coin<Lpn>,
        by: Timestamp,
        lease: Addr,
    ) -> ContractResult<Receipt<Lpn>> {
        self.debug_check_start_due_before(by, "before the 'repay-by' time");

        let (principal_due, total_interest_due) = self.load_lpp_loan(lease.clone())?
            .ok_or(ContractError::LoanClosed())
            .map(|resp| (resp.principal_due, resp.interest_due))?;

        let mut paid = Receipt::default();

        let change = self.repay_margin_interest(principal_due, by, payment, &mut paid, true);

        dbg!(paid.previous_margin_paid());

        if change.is_zero() {
            return Ok(paid);
        }

        let interest_overdue = self.load_loan_interest_due(lease, self.current_period.start())?;

        let interest_due = total_interest_due - dbg!(interest_overdue);

        let loan_payment = if interest_overdue <= change && self.current_period.zero_length() {
            paid.pay_previous_interest(interest_overdue);
            self.open_next_period();

            let surplus = change - interest_overdue;

            interest_overdue + self.repay_margin_interest(
                principal_due, by, surplus, &mut paid, false)
        } else {
            change
        };
        if loan_payment.is_zero() {
            // in practice not possible, but in theory it is if two consecutive repayments are received
            // with the same 'by' time.
            // TODO return profit.batch + lpp.batch
            return Ok(paid);
        }
        // TODO handle any surplus left after the repayment, options:
        //  - query again the lpp on the interest due by now + calculate the max repayment by now + send the surplus to the customer, or
        //  - [better separation of responsabilities, need of a 'reply' contract entry] pay lpp and once the surplus is received send it to the customer, or
        //  - [better separation of responsabilities + low trx cost] keep the surplus in the lease and send it back on lease.close
        //  - [better separation of responsabilities + even lower trx cost] include the remaining interest due up to this moment in the Lpp.query_loan response
        //  and send repayment amount up to the principal + interest due. The remainder is left in the lease

        // TODO For repayment, use not only the amount received but also the amount present in the lease. The latter may have been left as a surplus from a previous payment.
        self.lpp.repay_loan_req(dbg!(loan_payment))?;

        paid.pay_current_interest(dbg!(interest_due));

        if dbg!(total_interest_due) <= loan_payment {
            paid.pay_principal(principal_due, loan_payment - total_interest_due);
        }

        dbg!(&paid);

        assert_eq!(
            paid.previous_margin_paid() + paid.current_margin_paid() +
                paid.previous_interest_paid() + paid.current_interest_paid() +
                paid.principal_paid(),
            payment,
        );

        Ok(paid)
    }

    pub(crate) fn state(
        &self,
        now: Timestamp,
        lease: impl Into<Addr>,
    ) -> ContractResult<Option<State<Lpn>>> {
        self.debug_check_start_due_before(now, "in the past of");

        let loan_resp = self.load_lpp_loan(lease)?;
        Ok(loan_resp.map(|loan_state| self.merge_state_with(loan_state, now)))
    }

    fn load_loan_interest_due(
        &self,
        lease: impl Into<Addr>,
        by: Timestamp,
    ) -> ContractResult<Coin<Lpn>> {
        let interest = self
            .lpp
            .loan_outstanding_interest(lease, by)
            .map_err(ContractError::from)?;
        Ok(interest.ok_or(ContractError::LoanClosed())?.0)
    }

    fn load_lpp_loan(&self, lease: impl Into<Addr>) -> ContractResult<QueryLoanResponse<Lpn>> {
        self.lpp.loan(lease).map_err(ContractError::from)
    }

    fn repay_margin_interest(
        &mut self,
        principal_due: Coin<Lpn>,
        by: Timestamp,
        payment: Coin<Lpn>,
        paid: &mut Receipt<Lpn>,
        previous: bool,
    ) -> Coin<Lpn> {
        let (period, change) = dbg!(self.current_period).pay(principal_due, payment, dbg!(by));
        self.current_period = period;

        {
            let margin_paid = dbg!(payment - dbg!(change));

            if previous {
                paid.pay_previous_margin(margin_paid);
            } else {
                paid.pay_current_margin(margin_paid);
            }
        }

        // TODO send payment - change to profit
        change
    }

    fn open_next_period(&mut self) {
        debug_assert!(self.current_period.zero_length());

        self.current_period = InterestPeriod::with_interest(self.annual_margin_interest)
            .from(self.current_period.till())
            .spanning(self.interest_due_period);
    }

    fn merge_state_with(&self, loan_state: LoanResponse<Lpn>, now: Timestamp) -> State<Lpn> {
        let principal_due = loan_state.principal_due;
        let margin_interest_period = self
            .current_period
            .spanning(Duration::between(self.current_period.start(), now));

        let margin_interest_due = margin_interest_period.interest(principal_due);
        let interest_due = loan_state.interest_due + margin_interest_due;
        State {
            annual_interest: loan_state.annual_interest_rate + self.annual_margin_interest,
            principal_due,
            interest_due,
        }
    }

    fn debug_check_start_due_before(&self, when: Timestamp, when_descr: &str) {
        debug_assert!(
            self.current_period.start() <= when,
            "The current due period {}, should begin {} {}",
            self.current_period.start(),
            when_descr,
            when
        );
    }
}

impl<Lpn, Lpp> From<Loan<Lpn, Lpp>> for Batch
    where
        Lpp: Into<Batch>,
{
    fn from(loan: Loan<Lpn, Lpp>) -> Self {
        loan.lpp.into()
    }
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::{Addr, Timestamp};
    use serde::{Serialize, Deserialize};
    use finance::coin::{Amount, Coin};
    use finance::currency::{Currency, Nls, Usdc};
    use finance::duration::{Duration, Units as DurationUnits};
    use finance::percent::Percent;
    use finance::ratio::Ratio;
    use lpp::msg::{BalanceResponse, LoanResponse, LppBalanceResponse, OutstandingInterest, PriceResponse, QueryConfigResponse, QueryLoanOutstandingInterestResponse, QueryLoanResponse, QueryQuoteResponse, RewardsResponse};
    use lpp::stub::{Lpp, LppRef};
    use lpp::error::ContractError as LppError;
    use platform::bank::BankAccountView;
    use platform::batch::Batch;
    use platform::error::Result as PlatformResult;
    use crate::loan::{Loan, LoanDTO, Receipt};

    const MARGIN_INTEREST_RATE: Percent = Percent::from_permille(500); // 50%
    const LEASE_START: Timestamp = Timestamp::from_nanos(100);

    type TestCurrency = Usdc;
    type LppResult<T> = Result<T, LppError>;

    #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
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

    #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
    struct LppLocalStub {
        loan: Option<LoanResponse<TestCurrency>>,
    }

    // TODO define a MockLpp trait to avoid implementing Lpp-s from scratch
    impl Lpp<TestCurrency> for LppLocalStub {
        fn open_loan_req(&mut self, _amount: Coin<TestCurrency>) -> LppResult<()> {
            unreachable!()
        }

        fn open_loan_resp(&self, _resp: cosmwasm_std::Reply) -> LppResult<()> {
            unreachable!()
        }

        fn repay_loan_req(&mut self, repayment: Coin<TestCurrency>) -> LppResult<()> {
            // let loan = self.loan.as_mut().ok_or(LppError::NoLoan {})?;
            //
            // loan.principal_due -= repayment;
            //
            // loan.interest_due -= repayment.min(loan.interest_due);
            //
            // Ok(())

            Ok(())
        }

        fn loan(&self, _lease: impl Into<Addr>) -> LppResult<QueryLoanResponse<TestCurrency>> {
            Ok(self.loan.clone())
        }

        fn loan_outstanding_interest(
            &self,
            _lease: impl Into<Addr>,
            by: Timestamp,
        ) -> LppResult<QueryLoanOutstandingInterestResponse<TestCurrency>> {
            Ok(self.loan.as_ref().map(
                |loan| OutstandingInterest(loan.interest_due)
            ))
        }

        fn quote(&self, _amount: Coin<TestCurrency>) -> LppResult<QueryQuoteResponse> {
            unreachable!()
        }

        fn lpp_balance(&self) -> LppResult<LppBalanceResponse<TestCurrency>> {
            unreachable!()
        }

        fn nlpn_price(&self) -> LppResult<PriceResponse<TestCurrency>> {
            unreachable!()
        }

        fn config(&self) -> LppResult<QueryConfigResponse> {
            unreachable!()
        }

        fn nlpn_balance(&self, _lender: impl Into<Addr>) -> LppResult<BalanceResponse> {
            unreachable!()
        }

        fn rewards(&self, _lender: impl Into<Addr>) -> LppResult<RewardsResponse> {
            unreachable!()
        }
    }

    impl From<LppLocalStub> for Batch {
        fn from(_: LppLocalStub) -> Self {
            Batch::default()
        }
    }

    fn create_loan(addr: &str, loan_response: Option<LoanResponse<TestCurrency>>) -> Loan<TestCurrency, LppLocalStub>
    {
        let lpp_ref = LppRef::unchecked::<_, Nls>(addr);

        let loan_dto = LoanDTO::new(
            LEASE_START,
            lpp_ref,
            MARGIN_INTEREST_RATE,
            Duration::from_secs((Duration::YEAR.nanos() / 1000000) as u32),
            Duration::from_secs(0),
        );

        Loan::from_dto(loan_dto, LppLocalStub { loan: loan_response })
    }

    #[test]
    fn partial_margin_repay() {
        let addr = "unused_addr";

        let lease_amount = 1000;
        let lease_coin = coin(lease_amount);

        let interest_amount = lease_amount / 2;
        let interest_coin = coin(interest_amount);

        let repay_amount = lease_amount / 4;
        let repay_coin = coin(repay_amount);

        let interest_rate = Percent::from_permille(50);

        // LPP loan
        let loan_resp = LoanResponse {
            principal_due: lease_coin,
            interest_due: interest_coin,
            annual_interest_rate: interest_rate,
            interest_paid: Timestamp::from_nanos(0),
        };

        let loan = create_loan(addr, Some(loan_resp));

        let receipt = loan.repay(
            repay_coin,
            LEASE_START + Duration::YEAR,
            Addr::unchecked(addr),
        )
            .unwrap()
            .receipt;

        assert_eq!(
            receipt,
            {
                let mut receipt = Receipt::default();

                receipt.pay_previous_margin(repay_coin);

                receipt
            },
        );
    }

    #[test]
    fn partial_principal_repay() {
        let addr = "unused_addr";

        let lease_amount = 1000;
        let lease_coin = coin(lease_amount);

        let repay_amount = lease_amount / 2;
        let repay_coin = coin(repay_amount);

        let interest_rate = Percent::from_permille(500);

        // LPP loan
        let loan = LoanResponse {
            principal_due: lease_coin,
            interest_due: coin(0),
            annual_interest_rate: interest_rate,
            interest_paid: Timestamp::from_nanos(0),
        };

        let lease = create_loan(addr, Some(loan));

        let receipt = lease.repay(
            repay_coin,
            LEASE_START,
            Addr::unchecked(addr),
        )
            .unwrap()
            .receipt;

        assert_eq!(
            receipt,
            {
                let mut receipt = Receipt::default();

                receipt.pay_principal(lease_coin, repay_coin);

                receipt
            },
        );
    }

    #[test]
    fn partial_interest_principal_repay() {
        let addr = "unused_addr";

        let lease_amount = 1000;
        let lease_coin = coin(lease_amount);

        let interest_amount = lease_amount / 4;
        let interest_coin = coin(interest_amount);

        let repay_amount = lease_amount;
        let repay_coin = coin(repay_amount);

        let interest_rate = Percent::from_permille(250);

        // LPP loan
        let loan_resp = LoanResponse {
            principal_due: lease_coin,
            interest_due: interest_coin,
            annual_interest_rate: interest_rate,
            interest_paid: Timestamp::from_nanos(0),
        };

        let loan = create_loan(addr, Some(loan_resp));

        let receipt = loan.repay(
            repay_coin,
            LEASE_START + Duration::YEAR,
            Addr::unchecked(addr),
        )
            .unwrap()
            .receipt;

        assert_eq!(
            receipt,
            {
                let mut receipt = Receipt::default();

                receipt.pay_previous_margin(coin(lease_amount * Into::<Amount>::into(MARGIN_INTEREST_RATE.parts()) / 1000));

                receipt.pay_previous_interest(interest_coin);

                receipt.pay_principal(lease_coin, repay_coin - interest_coin - receipt.previous_margin_paid());

                receipt
            },
        );
    }

    #[test]
    fn full_principal_repay() {
        let addr = "unused_addr";

        let lease_amount = 1000;
        let lease_coin = coin(lease_amount);

        let interest_rate = Percent::from_permille(50);

        // LPP loan
        let loan = LoanResponse {
            principal_due: lease_coin,
            interest_due: coin(0),
            annual_interest_rate: interest_rate,
            interest_paid: Timestamp::from_nanos(0),
        };

        let lease = create_loan(addr, Some(loan));

        let receipt = lease.repay(
            lease_coin,
            LEASE_START,
            Addr::unchecked(addr),
        )
            .unwrap()
            .receipt;

        assert_eq!(
            receipt,
            {
                let mut receipt = Receipt::default();

                receipt.pay_principal(lease_coin, lease_coin);

                receipt
            },
        );
    }

    fn coin(a: u128) -> Coin<TestCurrency> {
        Coin::<TestCurrency>::new(a)
    }
}
