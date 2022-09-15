use std::{fmt::Debug, marker::PhantomData};

use cosmwasm_std::{Addr, Reply, Timestamp};
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
    stub::{Lpp as LppTrait, LppBatch, LppRef},
};
use platform::batch::Batch;

use crate::error::{ContractError, ContractResult};

pub use self::state::State;
pub(crate) use self::{
    liability::LiabilityStatus, open::Receipt as OpenReceipt, repay::Receipt as RepayReceipt,
};

mod liability;
mod open;
mod repay;
mod state;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub(crate) struct LoanDTO {
    annual_margin_interest: Percent,
    lpp: LppRef,
    interest_due_period: Duration,
    grace_period: Duration,
    current_period: InterestPeriod<Units, Percent>,
}

impl LoanDTO {
    fn new_raw(
        annual_margin_interest: Percent,
        lpp: LppRef,
        interest_due_period: Duration,
        grace_period: Duration,
        current_period: InterestPeriod<Units, Percent>,
    ) -> Self {
        debug_assert!(grace_period < interest_due_period);
        Self {
            annual_margin_interest,
            lpp,
            interest_due_period,
            grace_period,
            current_period,
        }
    }
    pub(crate) fn new(
        start: Timestamp,
        lpp: LppRef,
        annual_margin_interest: Percent,
        interest_due_period: Duration,
        grace_period: Duration,
    ) -> ContractResult<Self> {
        if grace_period >= interest_due_period {
            Err(ContractError::InvalidParameters(format!(
                "The grace period, currently {}, must be shorter that an interest period, currently {}, to avoid overlapping",
                grace_period,
                interest_due_period,
            )))
        } else {
            Ok(Self::new_raw(
                annual_margin_interest,
                lpp,
                interest_due_period,
                grace_period,
                InterestPeriod::with_interest(annual_margin_interest)
                    .from(start)
                    .spanning(interest_due_period),
            ))
        }
    }

    pub(crate) fn lpp(&self) -> &LppRef {
        &self.lpp
    }
}

pub struct Loan<Lpn, Lpp> {
    annual_margin_interest: Percent,
    lpn: PhantomData<Lpn>,
    lpp: Lpp,
    interest_due_period: Duration,
    grace_period: Duration,
    current_period: InterestPeriod<Units, Percent>,
}

impl<Lpn, Lpp> Loan<Lpn, Lpp>
where
    Lpp: LppTrait<Lpn>,
    Lpn: Currency + Debug,
{
    pub(super) fn from_dto(dto: LoanDTO, lpp: Lpp) -> Self {
        let res = Self {
            annual_margin_interest: dto.annual_margin_interest,
            lpn: PhantomData,
            lpp,
            interest_due_period: dto.interest_due_period,
            grace_period: dto.grace_period,
            current_period: dto.current_period,
        };
        debug_assert!(res.grace_period < res.interest_due_period);
        res
    }

    pub(super) fn into_dto(self) -> (LoanDTO, Batch) {
        let LppBatch { lpp_ref, batch } = self.lpp.into();
        let dto = LoanDTO::new_raw(
            self.annual_margin_interest,
            lpp_ref,
            self.interest_due_period,
            self.grace_period,
            self.current_period,
        );
        (dto, batch)
    }

    pub(crate) fn open_loan_req(&mut self, amount: Coin<Lpn>) -> ContractResult<()> {
        self.lpp.open_loan_req(amount)?;

        Ok(())
    }

    pub(crate) fn open_loan_resp(&mut self, resp: Reply) -> ContractResult<OpenReceipt<Lpn>> {
        let response = self.lpp.open_loan_resp(resp)?;

        Ok(OpenReceipt {
            annual_interest_rate: response.annual_interest_rate + self.annual_margin_interest,
            borrowed: response.principal_due,
        })
    }

    pub(crate) fn grace_period_end(&self) -> Timestamp {
        self.current_period.till() + self.grace_period
    }

    pub(crate) fn repay(
        &mut self,
        payment: Coin<Lpn>,
        by: Timestamp,
        lease: Addr,
    ) -> ContractResult<RepayReceipt<Lpn>> {
        self.debug_check_start_due_before(by, "before the 'repay-by' time");
        self.debug_check_before_period_end(by);

        let (principal_due, total_interest_due) = self
            .load_lpp_loan(lease.clone())?
            .ok_or(ContractError::LoanClosed())
            .map(|resp| (resp.principal_due, resp.interest_due))?;

        let mut receipt = RepayReceipt::default();

        let (change, mut loan_payment) = if self.overdue_at(by) {
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
            loan_payment += self.repay_current_period(
                by,
                principal_due,
                total_interest_due,
                &mut receipt,
                change,
            );

            // TODO uncomment when issue #13 is solved
            // debug_assert_eq!(
            //     loan_payment,
            //     receipt.previous_interest_paid()
            //         + receipt.current_interest_paid()
            //         + receipt.principal_paid(),
            // );

            // TODO remove when issue #13 is solved
            debug_assert!(
                receipt.previous_interest_paid()
                    + receipt.current_interest_paid()
                    + receipt.principal_paid()
                    <= loan_payment
            );
        }

        if loan_payment.is_zero() {
            // in practice not possible, but in theory it is if two consecutive repayments are received
            // with the same 'by' time.
            // TODO return profit.batch + lpp.batch
            return Ok(receipt);
        }

        // TODO handle any surplus left after the repayment, options:
        //  - query again the lpp on the interest due by now + calculate the max repayment by now + send the surplus to the customer, or
        //  - [better separation of responsabilities, need of a 'reply' contract entry] pay lpp and once the surplus is received send it to the customer, or
        //  - [better separation of responsabilities + low trx cost] keep the surplus in the lease and send it back on lease.close
        //  - [better separation of responsabilities + even lower trx cost] include the remaining interest due up to this moment in the Lpp.query_loan response
        //  and send repayment amount up to the principal + interest due. The remainder is left in the lease

        // TODO For repayment, use not only the amount received but also the amount present in the lease. The latter may have been left as a surplus from a previous payment.
        self.lpp.repay_loan_req(loan_payment)?;

        // TODO uncomment when issue #13 is solved
        // debug_assert_eq!(receipt.total(), payment);

        // TODO remove when issue #13 is solved
        debug_assert!(receipt.total() <= payment);

        Ok(receipt)
    }

    pub(crate) fn state(&self, now: Timestamp, lease: Addr) -> ContractResult<Option<State<Lpn>>> {
        self.debug_check_start_due_before(now, "in the past of");

        let loan_state = if let Some(loan) = self.load_lpp_loan(lease.clone())? {
            loan
        } else {
            return Ok(None);
        };

        let principal_due = loan_state.principal_due;

        let margin_interest_overdue_period = if self.overdue_at(now) {
            self.current_period
        } else {
            self.current_period.spanning(Duration::default())
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
        let current_interest_due = loan_state.interest_due - previous_interest_due;

        Ok(Some(State {
            annual_interest: loan_state.annual_interest_rate,
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
        let (prev_margin_paid, change) = self.repay_margin_interest(principal_due, by, payment);

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
        total_interest_due: Coin<Lpn>,
        receipt: &mut RepayReceipt<Lpn>,
        change: Coin<Lpn>,
    ) -> Coin<Lpn> {
        let mut loan_repay = Coin::default();

        let (curr_margin_paid, mut change) = self.repay_margin_interest(principal_due, by, change);

        receipt.pay_current_margin(curr_margin_paid);

        {
            let curr_interest_paid =
                change.min(total_interest_due - receipt.previous_interest_paid());

            change -= curr_interest_paid;

            loan_repay += curr_interest_paid;

            receipt.pay_current_interest(curr_interest_paid);
        }

        {
            let principal_paid = change;

            loan_repay += principal_paid;

            receipt.pay_principal(principal_due, principal_paid);
        }

        loan_repay
    }

    fn repay_margin_interest(
        &mut self,
        principal_due: Coin<Lpn>,
        by: Timestamp,
        payment: Coin<Lpn>,
    ) -> (Coin<Lpn>, Coin<Lpn>) {
        let (period, change) = self.current_period.pay(principal_due, payment, by);
        self.current_period = period;

        // TODO send payment - change to profit
        (payment - change, change)
    }

    fn open_next_period(&mut self) {
        debug_assert!(self.current_period.zero_length());

        self.current_period = InterestPeriod::with_interest(self.annual_margin_interest)
            .from(self.current_period.till())
            .spanning(self.interest_due_period);
    }

    fn overdue_at(&self, when: Timestamp) -> bool {
        self.current_period.till() <= when
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
            when <= self.current_period.till() + self.interest_due_period,
            "Payment is tried at {}s which is not within the current or next period ending at {}s",
            when,
            self.current_period.till() + self.interest_due_period,
        );
    }
}

impl<Lpn, Lpp> From<Loan<Lpn, Lpp>> for LppBatch
where
    Lpp: Into<LppBatch>,
{
    fn from(loan: Loan<Lpn, Lpp>) -> Self {
        loan.lpp.into()
    }
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::{Addr, Timestamp};
    use serde::{Deserialize, Serialize};

    use finance::coin::Coin;
    use finance::currency::{Currency, Nls, Usdc};
    use finance::duration::Duration;
    use finance::fraction::Fraction;
    use finance::interest::InterestPeriod;
    use finance::percent::Percent;
    use lpp::error::ContractError as LppError;
    use lpp::msg::{
        BalanceResponse, LoanResponse, LppBalanceResponse, OutstandingInterest, PriceResponse,
        QueryConfigResponse, QueryLoanOutstandingInterestResponse, QueryLoanResponse,
        QueryQuoteResponse, RewardsResponse,
    };
    use lpp::stub::{Lpp, LppBatch, LppRef};
    use platform::bank::BankAccountView;
    use platform::error::Result as PlatformResult;

    use crate::loan::repay::Receipt as RepayReceipt;
    use crate::loan::{Loan, LoanDTO};
    use crate::repay_id::ReplyId;

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
    struct LppLocalStub {
        loan: Option<LoanResponse<TestCurrency>>,
    }

    // TODO define a MockLpp trait to avoid implementing Lpp-s from scratch
    impl Lpp<TestCurrency> for LppLocalStub {
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

    impl From<LppLocalStub> for LppBatch {
        fn from(_: LppLocalStub) -> Self {
            unreachable!()
        }
    }

    fn create_loan(
        addr: &str,
        loan_response: Option<LoanResponse<TestCurrency>>,
    ) -> Loan<TestCurrency, LppLocalStub> {
        let lpp_ref = LppRef::unchecked::<_, Nls>(addr, Some(ReplyId::OpenLoanReq.into()));

        let loan_dto = LoanDTO::new(
            LEASE_START,
            lpp_ref,
            MARGIN_INTEREST_RATE,
            Duration::YEAR,
            Duration::from_secs(0),
        )
        .unwrap();

        Loan::from_dto(
            loan_dto,
            LppLocalStub {
                loan: loan_response,
            },
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
            interest_due: coin(0),
            annual_interest_rate: interest_rate,
            interest_paid: Timestamp::from_nanos(0),
        };

        let mut loan = create_loan(addr, Some(loan_resp));

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

        let interest_rate = Percent::from_permille(0);

        // LPP loan
        let loan_resp = LoanResponse {
            principal_due: lease_coin,
            interest_due: coin(0),
            annual_interest_rate: interest_rate,
            interest_paid: Timestamp::from_nanos(0),
        };

        let mut loan = create_loan(addr, Some(loan_resp));

        let receipt = loan
            .repay(
                repay_coin,
                LEASE_START + Duration::from_nanos(Duration::YEAR.nanos() - 1),
                Addr::unchecked(addr),
            )
            .unwrap();

        assert_eq!(receipt, {
            let mut receipt = RepayReceipt::default();

            receipt.pay_current_margin(repay_coin);

            receipt
        },);
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
            interest_due: coin(lease_amount / 2),
            annual_interest_rate: interest_rate,
            interest_paid: Timestamp::from_nanos(0),
        };

        let mut loan = create_loan(addr, Some(loan_resp));
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
            interest_due: interest_coin,
            annual_interest_rate: interest_rate,
            interest_paid: Timestamp::from_nanos(0),
        };

        let mut loan = create_loan(addr, Some(loan_resp));

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
            interest_due: coin(0),
            annual_interest_rate: interest_rate,
            interest_paid: Timestamp::from_nanos(0),
        };

        let mut lease = create_loan(addr, Some(loan));

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
            interest_due: interest_coin,
            annual_interest_rate: interest_rate,
            interest_paid: Timestamp::from_nanos(0),
        };

        let mut loan = create_loan(addr, Some(loan_resp));

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
            interest_due: coin(0),
            annual_interest_rate: interest_rate,
            interest_paid: Timestamp::from_nanos(0),
        };

        let mut lease = create_loan(addr, Some(loan));

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
            interest_due: interest(period, principal_due, interest_rate),
            annual_interest_rate: interest_rate,
            interest_paid: LEASE_START,
        };

        let loan = create_loan(LEASE_ADDRESS, Some(loan_resp.clone()));
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
