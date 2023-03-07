use std::{fmt::Debug, marker::PhantomData};

use serde::{Deserialize, Serialize};

use finance::{
    coin::Coin,
    currency::Currency,
    duration::Duration,
    interest::InterestPeriod,
    percent::{Percent, Units},
};
use lpp::{
    msg::QueryLoanResponse,
    stub::{
        lender::{LppLender as LppLenderTrait, LppLenderRef},
        LppBatch,
    },
};
use platform::batch::Batch;
use profit::stub::{Profit as ProfitTrait, ProfitBatch, ProfitRef};
use sdk::cosmwasm_std::{Addr, Timestamp};

use crate::{
    api::InterestPaymentSpec,
    error::{ContractError, ContractResult},
};

pub use self::state::State;
pub(crate) use self::{liability::LiabilityStatus, repay::Receipt as RepayReceipt};

mod liability;
mod repay;
mod state;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub(crate) struct LoanDTO {
    annual_margin_interest: Percent,
    lpp: LppLenderRef,
    interest_payment_spec: InterestPaymentSpec,
    current_period: InterestPeriod<Units, Percent>,
    profit: ProfitRef,
}

impl LoanDTO {
    fn new(
        annual_margin_interest: Percent,
        lpp: LppLenderRef,
        interest_payment_spec: InterestPaymentSpec,
        current_period: InterestPeriod<Units, Percent>,
        profit: ProfitRef,
    ) -> Self {
        Self {
            annual_margin_interest,
            lpp,
            interest_payment_spec,
            current_period,
            profit,
        }
    }

    pub(crate) fn annual_margin_interest(&self) -> Percent {
        self.annual_margin_interest
    }

    pub(crate) fn lpp(&self) -> &LppLenderRef {
        &self.lpp
    }

    pub(crate) fn profit(&self) -> &ProfitRef {
        &self.profit
    }
}

pub struct Loan<Lpn, Lpp, Profit> {
    annual_margin_interest: Percent,
    lpn: PhantomData<Lpn>,
    lpp: Lpp,
    interest_payment_spec: InterestPaymentSpec,
    current_period: InterestPeriod<Units, Percent>,
    profit: Profit,
}

impl<Lpn, Lpp, Profit> Loan<Lpn, Lpp, Profit>
where
    Lpn: Currency + Debug,
    Lpp: LppLenderTrait<Lpn>,
    Profit: ProfitTrait,
{
    pub(super) fn new(
        start: Timestamp,
        lpp: Lpp,
        annual_margin_interest: Percent,
        interest_payment_spec: InterestPaymentSpec,
        profit: Profit,
    ) -> Self {
        let current_period = Self::due_period(
            annual_margin_interest,
            start,
            interest_payment_spec.due_period(),
        );
        Self {
            annual_margin_interest,
            lpn: PhantomData,
            lpp,
            interest_payment_spec,
            current_period,
            profit,
        }
    }

    pub(super) fn from_dto(dto: LoanDTO, lpp: Lpp, profit: Profit) -> Self {
        {
            let annual_margin_interest = dto.annual_margin_interest;
            let interest_payment_spec = dto.interest_payment_spec;
            let current_period = dto.current_period;
            Self {
                annual_margin_interest,
                lpn: PhantomData,
                lpp,
                interest_payment_spec,
                current_period,
                profit,
            }
        }
    }

    pub(super) fn into_dto(self) -> (LoanDTO, Batch) {
        let LppBatch {
            lpp_ref,
            batch: lpp_batch,
        } = self.lpp.into();

        let ProfitBatch {
            profit_ref,
            batch: profit_batch,
        } = self.profit.into();

        let dto = LoanDTO::new(
            self.annual_margin_interest,
            lpp_ref,
            self.interest_payment_spec,
            self.current_period,
            profit_ref,
        );

        (dto, lpp_batch.merge(profit_batch))
    }

    pub(crate) fn grace_period_end(&self) -> Timestamp {
        self.current_period.till() + self.interest_payment_spec.grace_period()
    }

    pub(crate) fn repay(
        &mut self,
        payment: Coin<Lpn>,
        by: Timestamp,
        lease: Addr,
    ) -> ContractResult<RepayReceipt<Lpn>> {
        self.debug_check_start_due_before(by, "before the 'repay-by' time");
        self.debug_check_before_period_end(by);

        let (principal_due, loan_interest_due) = self
            .load_lpp_loan(lease.clone())?
            .ok_or(ContractError::LoanClosed())
            .map(|loan| (loan.principal_due, loan.interest_due(by)))?;

        let mut receipt = RepayReceipt::default();

        let (mut change, mut loan_payment) = if self.overdue_at(by) {
            self.repay_previous_period(payment, by, lease, principal_due, &mut receipt)?
        } else {
            (payment, Coin::default())
        };

        debug_assert_eq!(
            payment,
            change + receipt.previous_margin_paid() + receipt.previous_interest_paid()
        );
        debug_assert_eq!(loan_payment, receipt.previous_interest_paid());
        debug_assert!(!self.overdue_at(by) || change == Coin::default());

        if !self.overdue_at(by) {
            let current_period_paid;

            (change, current_period_paid) = self.repay_current_period(
                by,
                principal_due,
                loan_interest_due,
                &mut receipt,
                change,
            )?;

            loan_payment += current_period_paid;

            debug_assert_eq!(
                loan_payment,
                receipt.previous_interest_paid()
                    + receipt.current_interest_paid()
                    + receipt.principal_paid(),
            );
        }

        receipt.keep_change(change);

        if loan_payment.is_zero() {
            // in practice not possible, but in theory it is if two consecutive repayments are received
            // with the same 'by' time.
            return Ok(receipt);
        }

        // TODO handle any surplus left after the repayment, options:
        //  - query again the lpp on the interest due by now + calculate the max repayment by now + send the surplus to the customer, or
        //  - [better separation of responsabilities, need of a 'reply' contract entry] pay lpp and once the surplus is received send it to the customer, or
        //  - [better separation of responsabilities + low trx cost] keep the surplus in the lease and send it back on lease.close
        //  - [better separation of responsabilities + even lower trx cost] include the remaining interest due up to this moment in the Lpp.query_loan response
        //  and send repayment amount up to the principal + interest due. The remainder is left in the lease

        self.lpp.repay_loan_req(loan_payment)?;

        debug_assert_eq!(receipt.total(), payment);

        Ok(receipt)
    }

    pub(crate) fn state(&self, now: Timestamp, lease: Addr) -> ContractResult<Option<State<Lpn>>> {
        self.debug_check_start_due_before(now, "in the past of");

        let loan = if let Some(loan) = self.load_lpp_loan(lease.clone())? {
            loan
        } else {
            return Ok(None);
        };

        let principal_due = loan.principal_due;

        let margin_interest_overdue_period = if self.overdue_at(now) {
            self.current_period
        } else {
            self.due_period_from_with_length(
                self.current_period.till() - self.interest_payment_spec.due_period(),
                Duration::default(),
            )
        };

        let margin_interest_due_period = self
            .current_period
            .from(margin_interest_overdue_period.till())
            .spanning(Duration::between(
                margin_interest_overdue_period.till(),
                now,
            ));

        debug_assert_eq!(margin_interest_due_period.till(), now);

        let previous_margin_interest_due = margin_interest_overdue_period.interest(principal_due);
        let current_margin_interest_due = margin_interest_due_period.interest(principal_due);

        let previous_interest_due =
            self.load_loan_interest_due(lease, margin_interest_overdue_period.till())?;
        let current_interest_due = loan.interest_due(now) - previous_interest_due;

        Ok(Some(State {
            annual_interest: loan.annual_interest_rate,
            annual_interest_margin: self.annual_margin_interest,
            principal_due,
            previous_interest_due,
            current_interest_due,
            previous_margin_interest_due,
            current_margin_interest_due,
        }))
    }

    fn maybe_load_loan_interest_due(
        &self,
        lease: impl Into<Addr>,
        by: Timestamp,
    ) -> ContractResult<Option<Coin<Lpn>>> {
        let interest = self
            .lpp
            .loan_outstanding_interest(lease, by)
            .map_err(ContractError::from)?;

        Ok(interest.map(|interest| interest.0))
    }

    fn load_loan_interest_due(
        &self,
        lease: impl Into<Addr>,
        by: Timestamp,
    ) -> ContractResult<Coin<Lpn>> {
        self.maybe_load_loan_interest_due(lease, by)
            .and_then(|maybe_interest| maybe_interest.ok_or(ContractError::LoanClosed()))
    }

    fn load_lpp_loan(&self, lease: impl Into<Addr>) -> ContractResult<QueryLoanResponse<Lpn>> {
        self.lpp.loan(lease).map_err(ContractError::from)
    }

    fn repay_previous_period(
        &mut self,
        payment: Coin<Lpn>,
        by: Timestamp,
        lease: Addr,
        principal_due: Coin<Lpn>,
        receipt: &mut RepayReceipt<Lpn>,
    ) -> ContractResult<(Coin<Lpn>, Coin<Lpn>)> {
        let (prev_margin_paid, change) = self.repay_margin_interest(principal_due, by, payment)?;

        receipt.pay_previous_margin(prev_margin_paid);

        if change.is_zero() {
            return Ok((Coin::default(), Coin::default()));
        }

        debug_assert!(self.current_period.zero_length()); // no prev_margin due

        let previous_interest_due =
            self.load_loan_interest_due(lease, self.current_period.start())?;

        let previous_interest_paid = previous_interest_due.min(change);

        receipt.pay_previous_interest(previous_interest_paid);

        if previous_interest_paid == previous_interest_due {
            self.open_next_period();
        }

        Ok((change - previous_interest_paid, previous_interest_paid))
    }

    fn repay_current_period(
        &mut self,
        by: Timestamp,
        principal_due: Coin<Lpn>,
        loan_interest_due: Coin<Lpn>,
        receipt: &mut RepayReceipt<Lpn>,
        change: Coin<Lpn>,
    ) -> ContractResult<(Coin<Lpn>, Coin<Lpn>)> {
        let mut loan_repay = Coin::default();

        let (curr_margin_paid, mut change) =
            self.repay_margin_interest(principal_due, by, change)?;

        receipt.pay_current_margin(curr_margin_paid);

        {
            let curr_interest_paid =
                change.min(loan_interest_due - receipt.previous_interest_paid());

            change -= curr_interest_paid;

            loan_repay += curr_interest_paid;

            receipt.pay_current_interest(curr_interest_paid);
        }

        {
            let principal_paid = change.min(principal_due);

            change -= principal_paid;

            loan_repay += principal_paid;

            receipt.pay_principal(principal_due, principal_paid);
        }

        Ok((change, loan_repay))
    }

    fn repay_margin_interest(
        &mut self,
        principal_due: Coin<Lpn>,
        by: Timestamp,
        payment: Coin<Lpn>,
    ) -> ContractResult<(Coin<Lpn>, Coin<Lpn>)> {
        let (period, change) = self.current_period.pay(principal_due, payment, by);
        self.current_period = period;

        let paid = payment - change;

        if !paid.is_zero() {
            self.profit.send(paid);
        }

        Ok((paid, change))
    }

    fn open_next_period(&mut self) {
        debug_assert!(self.current_period.zero_length());

        self.current_period = self.due_period_from(self.current_period.till());
    }

    fn overdue_at(&self, when: Timestamp) -> bool {
        self.current_period.till() <= when
    }

    #[inline]
    fn due_period_from(&self, start: Timestamp) -> InterestPeriod<Units, Percent> {
        Self::due_period(
            self.annual_margin_interest,
            start,
            self.interest_payment_spec.due_period(),
        )
    }

    #[inline]
    fn due_period_from_with_length(
        &self,
        start: Timestamp,
        period: Duration,
    ) -> InterestPeriod<Units, Percent> {
        Self::due_period(self.annual_margin_interest, start, period)
    }

    fn due_period(
        margin_interest: Percent,
        start: Timestamp,
        period: Duration,
    ) -> InterestPeriod<Units, Percent> {
        InterestPeriod::with_interest(margin_interest)
            .from(start)
            .spanning(period)
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
    fn debug_check_before_period_end(&self, when: Timestamp) {
        debug_assert!(
            when <= self.current_period.till() + self.interest_payment_spec.due_period(),
            "Payment is tried at {}s which is not within the current or next period ending at {}s",
            when,
            self.current_period.till() + self.interest_payment_spec.due_period(),
        );
    }
}

#[cfg(test)]
mod tests {
    use serde::{Deserialize, Serialize};

    use finance::{
        coin::Coin, currency::Currency, duration::Duration, fraction::Fraction,
        interest::InterestPeriod, percent::Percent, test::currency::Usdc,
    };
    use lpp::{
        error::ContractError as LppError,
        msg::{
            LoanResponse, OutstandingInterest, QueryLoanOutstandingInterestResponse,
            QueryLoanResponse, QueryQuoteResponse,
        },
        stub::{
            lender::{LppLender, LppLenderRef},
            LppBatch,
        },
    };
    use platform::{bank::BankAccountView, error::Result as PlatformResult};
    use profit::stub::{Profit, ProfitBatch};
    use sdk::cosmwasm_std::{Addr, Timestamp};

    use crate::{
        api::InterestPaymentSpec,
        loan::{repay::Receipt as RepayReceipt, Loan},
    };

    const MARGIN_INTEREST_RATE: Percent = Percent::from_permille(500); // 50%
    const LEASE_START: Timestamp = Timestamp::from_nanos(100);

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
    struct LppLenderLocalStub {
        loan: Option<LoanResponse<TestCurrency>>,
    }

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
        ) -> LppResult<QueryLoanOutstandingInterestResponse<TestCurrency>> {
            Ok(self.loan.as_ref().map(|loan| {
                OutstandingInterest(interest(
                    Duration::between(loan.interest_paid, by),
                    loan.principal_due,
                    loan.annual_interest_rate,
                ))
            }))
        }

        fn quote(&self, _amount: Coin<TestCurrency>) -> LppResult<QueryQuoteResponse> {
            unreachable!()
        }
    }

    impl From<LppLenderLocalStub> for LppBatch<LppLenderRef> {
        fn from(_: LppLenderLocalStub) -> Self {
            unreachable!()
        }
    }

    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    struct ProfitLocalStub {}

    impl Profit for ProfitLocalStub {
        fn send<C>(&mut self, _coins: Coin<C>)
        where
            C: Currency,
        {
        }
    }

    impl From<ProfitLocalStub> for ProfitBatch {
        fn from(_: ProfitLocalStub) -> Self {
            unreachable!()
        }
    }

    fn create_loan(
        loan_response: Option<LoanResponse<TestCurrency>>,
    ) -> Loan<TestCurrency, LppLenderLocalStub, ProfitLocalStub> {
        Loan::new(
            LEASE_START,
            LppLenderLocalStub {
                loan: loan_response,
            },
            MARGIN_INTEREST_RATE,
            InterestPaymentSpec::new(Duration::YEAR, Duration::from_secs(0)),
            ProfitLocalStub {},
        )
    }

    #[test]
    fn partial_previous_margin_repay() {
        let addr = "unused_addr";

        let lease_amount = 1000;
        let lease_coin = coin(lease_amount);

        let repay_amount = lease_amount / 4;
        let repay_coin = coin(repay_amount);

        let interest_rate = Percent::from_permille(0);

        // LPP loan
        let loan_resp = LoanResponse {
            principal_due: lease_coin,
            annual_interest_rate: interest_rate,
            interest_paid: Timestamp::from_nanos(0),
        };

        let mut loan = create_loan(Some(loan_resp));

        let receipt = loan
            .repay(
                repay_coin,
                LEASE_START + Duration::YEAR + Duration::YEAR,
                Addr::unchecked(addr),
            )
            .unwrap();

        assert_eq!(receipt, {
            let mut receipt = RepayReceipt::default();

            receipt.pay_previous_margin(repay_coin);

            receipt
        },);
    }

    #[test]
    fn partial_current_margin_repay() {
        let addr = "unused_addr";

        let lease_amount = 1000;
        let lease_coin = coin(lease_amount);

        let repay_amount = lease_amount / 4;
        let repay_coin = coin(repay_amount);

        let interest_rate = Percent::from_permille(50);

        // LPP loan
        let loan_resp = LoanResponse {
            principal_due: lease_coin,
            annual_interest_rate: interest_rate,
            interest_paid: Timestamp::from_nanos(0),
        };

        let mut loan = create_loan(Some(loan_resp));

        let now = LEASE_START + Duration::from_nanos(Duration::YEAR.nanos() - 1);

        let receipt = loan.repay(repay_coin, now, Addr::unchecked(addr)).unwrap();

        assert_eq!(receipt, {
            let mut receipt = RepayReceipt::default();

            receipt.pay_current_margin(repay_coin);

            receipt
        },);

        let state = loan.state(now, Addr::unchecked(addr)).unwrap().unwrap();

        assert_eq!(state.previous_margin_interest_due, Coin::default());

        assert_eq!(state.previous_interest_due, Coin::default());
    }

    #[test]
    fn partial_previous_interest_repay() {
        let addr = "unused_addr";
        let addr_obj = Addr::unchecked(addr);

        let lease_amount = 1000;
        let lease_coin = coin(lease_amount);

        let repay_amount = lease_amount / 4;
        let repay_coin = coin(repay_amount);

        let interest_rate = Percent::from_permille(500);

        // LPP loan
        let loan_resp = LoanResponse {
            principal_due: lease_coin,
            annual_interest_rate: interest_rate,
            interest_paid: Timestamp::from_nanos(0),
        };

        let mut loan = create_loan(Some(loan_resp));
        let margin_interest = MARGIN_INTEREST_RATE.of(lease_coin);
        let end_of_due_period = LEASE_START + Duration::YEAR;
        {
            let mut exp_full_prev_margin = RepayReceipt::default();
            exp_full_prev_margin.pay_previous_margin(margin_interest);
            assert_eq!(
                exp_full_prev_margin,
                loan.repay(margin_interest, end_of_due_period, addr_obj.clone())
                    .unwrap()
            );
        }

        {
            let mut exp_receipt = RepayReceipt::default();
            exp_receipt.pay_previous_interest(repay_coin);
            assert_eq!(
                exp_receipt,
                loan.repay(repay_coin, end_of_due_period, addr_obj).unwrap()
            );
        }
    }

    #[test]
    fn full_previous_partial_current_interest_repay() {
        let addr = "unused_addr";

        let lease_amount = 1000;
        let lease_coin = coin(lease_amount);

        let interest_amount = lease_amount / 2;
        let interest_coin = coin(interest_amount);

        let repay_amount = lease_amount;
        let repay_coin = coin(repay_amount);

        let interest_rate = Percent::from_permille(500);

        // LPP loan
        let loan_resp = LoanResponse {
            principal_due: lease_coin,
            annual_interest_rate: interest_rate,
            interest_paid: Timestamp::from_nanos(0),
        };

        let mut loan = create_loan(Some(loan_resp));

        let receipt = loan
            .repay(
                repay_coin,
                LEASE_START + Duration::YEAR,
                Addr::unchecked(addr),
            )
            .unwrap();

        assert_eq!(receipt, {
            let mut receipt = RepayReceipt::default();

            receipt.pay_previous_margin(MARGIN_INTEREST_RATE.of(lease_coin));

            receipt.pay_previous_interest(interest_coin);

            receipt
        },);
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
            annual_interest_rate: interest_rate,
            interest_paid: Timestamp::from_nanos(0),
        };

        let mut lease = create_loan(Some(loan));

        let receipt = lease
            .repay(repay_coin, LEASE_START, Addr::unchecked(addr))
            .unwrap();

        assert_eq!(receipt, {
            let mut receipt = RepayReceipt::default();

            receipt.pay_principal(lease_coin, repay_coin);

            receipt
        },);
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
            annual_interest_rate: interest_rate,
            interest_paid: Timestamp::from_nanos(0),
        };

        let mut loan = create_loan(Some(loan_resp));

        let receipt = loan
            .repay(
                repay_coin,
                LEASE_START + Duration::YEAR,
                Addr::unchecked(addr),
            )
            .unwrap();

        assert_eq!(receipt, {
            let mut receipt = RepayReceipt::default();

            receipt.pay_previous_margin(MARGIN_INTEREST_RATE.of(lease_coin));

            receipt.pay_previous_interest(interest_coin);

            receipt.pay_principal(
                lease_coin,
                repay_coin - interest_coin - receipt.previous_margin_paid(),
            );

            receipt
        },);
    }

    #[test]
    fn full_principal_repay() {
        let addr = "unused_addr";

        let lease_amount = 1000;
        let lease_coin = coin(lease_amount);

        let interest_rate = Percent::from_permille(500);

        // LPP loan
        let loan = LoanResponse {
            principal_due: lease_coin,
            annual_interest_rate: interest_rate,
            interest_paid: Timestamp::from_nanos(0),
        };

        let mut lease = create_loan(Some(loan));

        let receipt = lease
            .repay(lease_coin, LEASE_START, Addr::unchecked(addr))
            .unwrap();

        assert_eq!(receipt, {
            let mut receipt = RepayReceipt::default();

            receipt.pay_principal(lease_coin, lease_coin);

            receipt
        },);
    }

    fn interest<Lpn>(period: Duration, principal_due: Coin<Lpn>, rate: Percent) -> Coin<Lpn>
    where
        Lpn: Currency,
    {
        InterestPeriod::with_interest(rate)
            .spanning(period)
            .interest(principal_due)
    }

    fn interests<Lpn>(
        paid: Timestamp,
        now: Timestamp,
        principal_due: Coin<Lpn>,
        rate: Percent,
    ) -> (Coin<Lpn>, Coin<Lpn>)
    where
        Lpn: Currency,
    {
        (
            interest(
                if now < LEASE_START + Duration::YEAR {
                    Duration::default()
                } else {
                    Duration::between(paid, LEASE_START + Duration::YEAR)
                },
                principal_due,
                rate,
            ),
            interest(
                Duration::between(
                    if now < LEASE_START + Duration::YEAR {
                        paid
                    } else {
                        LEASE_START + Duration::YEAR
                    },
                    now,
                ),
                principal_due,
                rate,
            ),
        )
    }

    fn margin_interests<Lpn>(
        paid: Timestamp,
        now: Timestamp,
        principal_due: Coin<Lpn>,
    ) -> (Coin<Lpn>, Coin<Lpn>)
    where
        Lpn: Currency,
    {
        interests(paid, now, principal_due, MARGIN_INTEREST_RATE)
    }

    fn test_state(period: Duration) {
        const LEASE_ADDRESS: &str = "";

        let principal_due = coin(10000);

        let interest_rate = Percent::from_permille(25);

        let loan_resp = LoanResponse {
            principal_due,
            annual_interest_rate: interest_rate,
            interest_paid: LEASE_START,
        };

        let loan = create_loan(Some(loan_resp.clone()));
        let now = LEASE_START + period;

        let (expected_margin_overdue, expected_margin_due) =
            margin_interests(loan_resp.interest_paid, now, principal_due);

        let (expected_interest_overdue, expected_interest_due) = interests(
            loan_resp.interest_paid,
            now,
            principal_due,
            loan_resp.annual_interest_rate,
        );

        let res = loan
            .state(now, Addr::unchecked(LEASE_ADDRESS))
            .unwrap()
            .unwrap();

        assert_eq!(
            res.previous_margin_interest_due, expected_margin_overdue,
            "Got different margin overdue than expected!",
        );

        assert_eq!(
            res.current_margin_interest_due, expected_margin_due,
            "Got different margin due than expected!",
        );

        assert_eq!(
            res.previous_interest_due, expected_interest_overdue,
            "Got different interest overdue than expected!",
        );

        assert_eq!(
            res.current_interest_due, expected_interest_due,
            "Got different interest due than expected!",
        );
    }

    #[test]
    fn state_zero() {
        test_state(Duration::default())
    }

    #[test]
    fn state_day() {
        test_state(Duration::from_days(1))
    }

    #[test]
    fn state_year() {
        test_state(Duration::YEAR)
    }

    #[test]
    fn state_year_plus_day() {
        test_state(Duration::YEAR + Duration::from_days(1))
    }

    #[test]
    fn state_year_minus_day() {
        test_state(Duration::YEAR - Duration::from_days(1))
    }

    fn coin(a: u128) -> Coin<TestCurrency> {
        Coin::<TestCurrency>::new(a)
    }
}
