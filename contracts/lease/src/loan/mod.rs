use std::{fmt::Debug, marker::PhantomData};

use serde::{Deserialize, Serialize};

use finance::{
    coin::Coin,
    currency::Currency,
    duration::Duration,
    interest::InterestPeriod,
    percent::{Percent, Units},
};
use lpp::stub::{loan::LppLoan as LppLoanTrait, LppBatch, LppRef};
use platform::batch::Batch;
use profit::stub::{Profit as ProfitTrait, ProfitRef};
use sdk::cosmwasm_std::Timestamp;

use crate::{api::InterestPaymentSpec, error::ContractResult};

pub use self::state::State;
pub(crate) use self::{liability::LiabilityStatus, repay::Receipt as RepayReceipt};

mod liability;
mod repay;
mod state;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub(crate) struct LoanDTO {
    annual_margin_interest: Percent,
    lpp: LppRef,
    interest_payment_spec: InterestPaymentSpec,
    current_period: InterestPeriod<Units, Percent>,
    profit: ProfitRef,
}

impl LoanDTO {
    fn new(
        annual_margin_interest: Percent,
        lpp: LppRef,
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

    pub(crate) fn lpp(&self) -> &LppRef {
        &self.lpp
    }

    pub(crate) fn profit(&self) -> &ProfitRef {
        &self.profit
    }
}

pub struct Loan<Lpn, LppLoan> {
    annual_margin_interest: Percent,
    lpn: PhantomData<Lpn>,
    lpp_loan: LppLoan,
    interest_payment_spec: InterestPaymentSpec,
    current_period: InterestPeriod<Units, Percent>,
}

impl<Lpn, LppLoan> Loan<Lpn, LppLoan>
where
    Lpn: Currency + Debug,
    LppLoan: LppLoanTrait<Lpn>,
{
    pub(super) fn new(
        start: Timestamp,
        lpp_loan: LppLoan,
        annual_margin_interest: Percent,
        interest_payment_spec: InterestPaymentSpec,
    ) -> Self {
        let current_period = Self::due_period(
            annual_margin_interest,
            start,
            interest_payment_spec.due_period(),
        );
        Self {
            annual_margin_interest,
            lpn: PhantomData,
            lpp_loan,
            interest_payment_spec,
            current_period,
        }
    }

    pub(super) fn from_dto(dto: LoanDTO, lpp_loan: LppLoan) -> Self {
        {
            let annual_margin_interest = dto.annual_margin_interest;
            let interest_payment_spec = dto.interest_payment_spec;
            let current_period = dto.current_period;
            Self {
                annual_margin_interest,
                lpn: PhantomData,
                lpp_loan,
                interest_payment_spec,
                current_period,
            }
        }
    }

    pub(super) fn into_dto(self, profit: ProfitRef) -> (LoanDTO, Batch) {
        let LppBatch {
            lpp_ref,
            batch: lpp_messages,
        } = self.lpp_loan.into();

        let dto = LoanDTO::new(
            self.annual_margin_interest,
            lpp_ref,
            self.interest_payment_spec,
            self.current_period,
            profit,
        );

        (dto, lpp_messages)
    }

    pub(crate) fn grace_period_end(&self) -> Timestamp {
        self.current_period.till() + self.interest_payment_spec.grace_period()
    }

    /// Repay the loan interests and principal by the given timestamp.
    ///
    /// The time intervals are always open-ended!
    pub(crate) fn repay<Profit>(
        &mut self,
        payment: Coin<Lpn>,
        by: Timestamp,
        profit: &mut Profit,
    ) -> ContractResult<RepayReceipt<Lpn>>
    where
        Profit: ProfitTrait,
    {
        self.debug_check_start_due_before(by, "before the 'repay-by' time");
        self.debug_check_before_next_due_end(by);

        let mut receipt = RepayReceipt::default();

        let (change, loan_prev_period_payment) = if self.overdue_at(by) {
            self.repay_previous_period(payment, by, profit, &mut receipt)?
        } else {
            (payment, Coin::default())
        };
        debug_assert_eq!(payment, change + receipt.total());
        debug_assert_eq!(loan_prev_period_payment, receipt.previous_interest_paid());
        debug_assert!(!self.overdue_at(by) || change == Coin::default());

        let (change, loan_curr_period_payment) = if !self.overdue_at(by) {
            self.repay_current_period(change, by, profit, &mut receipt)?
        } else {
            (change, Coin::default())
        };
        debug_assert_eq!(payment, change + receipt.total());
        debug_assert_eq!(
            loan_curr_period_payment,
            receipt.current_interest_paid() + receipt.principal_paid(),
        );

        receipt.keep_change(change);
        debug_assert_eq!(payment, receipt.total());

        let loan_payment = loan_prev_period_payment + loan_curr_period_payment;
        if !loan_payment.is_zero() {
            // In theory, zero loan payment may occur if two consecutive repayments are executed within the same time.
            // In practice, that means two repayment transactions of the same lease enter the same block.
            self.lpp_loan.repay(by, loan_payment)?;
        }
        Ok(receipt)
    }

    pub(crate) fn state(&self, now: Timestamp) -> State<Lpn> {
        self.debug_check_start_due_before(now, "in the past of");

        let principal_due = self.lpp_loan.principal_due();

        let margin_interest_overdue_period = if self.overdue_at(now) {
            self.current_period
        } else {
            self.due_period_from_with_length(
                self.current_period.till() - self.interest_payment_spec.due_period(),
                Duration::default(),
            )
        };

        let margin_interest_due_period_start = self
            .current_period
            .start()
            .max(margin_interest_overdue_period.till());
        let margin_interest_due_period = self
            .current_period
            .from(margin_interest_due_period_start)
            .spanning(Duration::between(margin_interest_due_period_start, now));
        debug_assert_eq!(margin_interest_due_period.till(), now);

        let previous_margin_interest_due = margin_interest_overdue_period.interest(principal_due);
        let current_margin_interest_due = margin_interest_due_period.interest(principal_due);

        let previous_interest_due = self
            .lpp_loan
            .interest_due(margin_interest_overdue_period.till());
        let current_interest_due = self.lpp_loan.interest_due(now) - previous_interest_due;

        State {
            annual_interest: self.lpp_loan.annual_interest_rate(),
            annual_interest_margin: self.annual_margin_interest,
            principal_due,
            previous_interest_due,
            current_interest_due,
            previous_margin_interest_due,
            current_margin_interest_due,
        }
    }

    fn repay_previous_period<Profit>(
        &mut self,
        payment: Coin<Lpn>,
        by: Timestamp,
        profit: &mut Profit,
        receipt: &mut RepayReceipt<Lpn>,
    ) -> ContractResult<(Coin<Lpn>, Coin<Lpn>)>
    where
        Profit: ProfitTrait,
    {
        let (prev_margin_paid, change) =
            self.repay_margin_interest(self.lpp_loan.principal_due(), by, payment, profit)?;
        receipt.pay_previous_margin(prev_margin_paid);

        if change.is_zero() {
            return Ok((Coin::default(), Coin::default()));
        }

        debug_assert!(self.current_period.zero_length()); // no prev_margin due

        let previous_interest_due = self.lpp_loan.interest_due(self.current_period.till());
        let previous_interest_paid = previous_interest_due.min(change);
        receipt.pay_previous_interest(previous_interest_paid);

        if previous_interest_paid == previous_interest_due {
            self.open_next_period();
        }

        Ok((change - previous_interest_paid, previous_interest_paid))
    }

    fn repay_current_period<Profit>(
        &mut self,
        payment: Coin<Lpn>,
        by: Timestamp,
        profit: &mut Profit,
        receipt: &mut RepayReceipt<Lpn>,
    ) -> ContractResult<(Coin<Lpn>, Coin<Lpn>)>
    where
        Profit: ProfitTrait,
    {
        let mut loan_repay = Coin::default();

        let (curr_margin_paid, mut change) =
            self.repay_margin_interest(self.lpp_loan.principal_due(), by, payment, profit)?;

        receipt.pay_current_margin(curr_margin_paid);

        {
            let curr_interest_paid =
                change.min(self.lpp_loan.interest_due(by) - receipt.previous_interest_paid());

            change -= curr_interest_paid;

            loan_repay += curr_interest_paid;

            receipt.pay_current_interest(curr_interest_paid);
        }

        {
            let principal_paid = change.min(self.lpp_loan.principal_due());

            change -= principal_paid;

            loan_repay += principal_paid;

            receipt.pay_principal(self.lpp_loan.principal_due(), principal_paid);
        }

        Ok((change, loan_repay))
    }

    fn repay_margin_interest<Profit>(
        &mut self,
        principal_due: Coin<Lpn>,
        by: Timestamp,
        payment: Coin<Lpn>,
        profit: &mut Profit,
    ) -> ContractResult<(Coin<Lpn>, Coin<Lpn>)>
    where
        Profit: ProfitTrait,
    {
        let (period, change) = self.current_period.pay(principal_due, payment, by);
        self.current_period = period;

        let paid = payment - change;

        if !paid.is_zero() {
            profit.send(paid);
        }

        Ok((paid, change))
    }

    fn open_next_period(&mut self) {
        debug_assert!(self.current_period.zero_length());

        self.current_period = self.due_period_from(self.current_period.till());
    }

    fn overdue_at(&self, when: Timestamp) -> bool {
        self.current_period.till() < when
    }

    fn due_period_from(&self, start: Timestamp) -> InterestPeriod<Units, Percent> {
        Self::due_period(
            self.annual_margin_interest,
            start,
            self.interest_payment_spec.due_period(),
        )
    }

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
            "The current due period starting at {}s, should begin {} {}s",
            self.current_period.start(),
            when_descr,
            when
        );
    }
    fn debug_check_before_next_due_end(&self, when: Timestamp) {
        let next_due_end = self.current_period.till() + self.interest_payment_spec.due_period();
        debug_assert!(
            when <= next_due_end,
            "Payment is tried at {}s which is not before the next period ending at {}s",
            when,
            next_due_end,
        );
    }
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::Timestamp;
    use currency::lpn::Usdc;
    use finance::{coin::Coin, duration::Duration, percent::Percent};
    use lpp::{
        error::ContractError as LppError,
        msg::LoanResponse,
        stub::{loan::LppLoan as LppLoanTrait, LppBatch, LppRef},
    };
    use platform::bank::LazySenderStub;
    use profit::stub::{ProfitRef, ProfitStub};
    use serde::{Deserialize, Serialize};

    use crate::api::InterestPaymentSpec;

    use super::Loan;

    const MARGIN_INTEREST_RATE: Percent = Percent::from_permille(50);
    const LOAN_INTEREST_RATE: Percent = Percent::from_permille(500);
    const LEASE_START: Timestamp = Timestamp::from_nanos(100);
    const PROFIT_ADDR: &str = "profit_addr";

    type Lpn = Usdc;
    type LppResult<T> = Result<T, LppError>;

    mod test_repay {
        use serde::{Deserialize, Serialize};

        use finance::{
            coin::{Amount, Coin, WithCoin},
            currency::{Currency, Group},
            duration::Duration,
            fraction::Fraction,
        };
        use lpp::msg::LoanResponse;
        use platform::{
            bank::{self, Aggregate, BalancesResult, BankAccountView},
            batch::Batch,
            error::Result as PlatformResult,
        };
        use sdk::cosmwasm_std::Timestamp;

        use crate::loan::{
            repay::Receipt as RepayReceipt,
            tests::{profit_stub, PROFIT_ADDR},
            Loan, State,
        };

        use super::{
            create_loan, Lpn, LppLoanLocal, LEASE_START, LOAN_INTEREST_RATE, MARGIN_INTEREST_RATE,
        };

        #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
        pub struct BankStub {
            balance: Amount,
        }

        impl BankAccountView for BankStub {
            fn balance<C>(&self) -> PlatformResult<Coin<C>>
            where
                C: Currency,
            {
                Ok(Coin::<C>::new(self.balance))
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

        #[test]
        fn two_periods_span_repay() {
            let principal = 1000;
            let delta_to_fully_paid = 50;
            let payment_at = LEASE_START + Duration::YEAR + Duration::YEAR;
            let one_year_margin = MARGIN_INTEREST_RATE.of(principal);
            let one_year_interest = LOAN_INTEREST_RATE.of(principal);

            let loan = LoanResponse {
                principal_due: principal.into(),
                annual_interest_rate: LOAN_INTEREST_RATE,
                interest_paid: LEASE_START,
            };

            let mut loan = create_loan(loan);
            {
                let repay_prev_margin = one_year_margin - delta_to_fully_paid;
                repay(
                    &mut loan,
                    repay_prev_margin,
                    state(
                        principal,
                        one_year_margin,
                        one_year_interest,
                        one_year_margin,
                        one_year_interest,
                    ),
                    receipt(principal, 0, repay_prev_margin, 0, 0, 0, 0),
                    payment_at,
                )
            }

            {
                let repay_fully_prev_margin_and_some_interest = one_year_interest;
                repay(
                    &mut loan,
                    repay_fully_prev_margin_and_some_interest,
                    state(
                        principal,
                        delta_to_fully_paid,
                        one_year_interest,
                        one_year_margin,
                        one_year_interest,
                    ),
                    receipt(
                        principal,
                        0,
                        delta_to_fully_paid,
                        repay_fully_prev_margin_and_some_interest - delta_to_fully_paid,
                        0,
                        0,
                        0,
                    ),
                    payment_at,
                )
            }

            {
                let repay_fully_prev_amount_and_some_curr_margin = one_year_margin;
                repay(
                    &mut loan,
                    repay_fully_prev_amount_and_some_curr_margin,
                    state(
                        principal,
                        0,
                        delta_to_fully_paid,
                        one_year_margin,
                        one_year_interest,
                    ),
                    receipt(
                        principal,
                        0,
                        0,
                        delta_to_fully_paid,
                        repay_fully_prev_amount_and_some_curr_margin - delta_to_fully_paid,
                        0,
                        0,
                    ),
                    payment_at,
                )
            }

            {
                let margin_due = delta_to_fully_paid;
                let surplus = delta_to_fully_paid;
                let full_repayment = margin_due + one_year_interest + principal + surplus;
                repay(
                    &mut loan,
                    full_repayment,
                    state(principal, 0, 0, delta_to_fully_paid, one_year_interest),
                    receipt(
                        principal,
                        principal,
                        0,
                        0,
                        delta_to_fully_paid,
                        one_year_interest,
                        surplus,
                    ),
                    payment_at,
                )
            }
        }

        #[test]
        fn partial_current_margin_repay() {
            let principal = 1000;
            let payment = MARGIN_INTEREST_RATE.of(principal) / 2;
            let now = LEASE_START + Duration::YEAR;

            let mut loan = create_loan(LoanResponse {
                principal_due: principal.into(),
                annual_interest_rate: LOAN_INTEREST_RATE,
                interest_paid: LEASE_START,
            });

            repay(
                &mut loan,
                payment,
                state(
                    principal,
                    0,
                    0,
                    MARGIN_INTEREST_RATE.of(principal),
                    LOAN_INTEREST_RATE.of(principal),
                ),
                receipt(principal, 0, 0, 0, payment, 0, 0),
                now,
            );
        }

        #[test]
        fn partial_previous_interest_repay() {
            let principal = 1000;
            let interest_payment = 43;
            let one_year_margin = MARGIN_INTEREST_RATE.of(principal);
            let one_year_interest = LOAN_INTEREST_RATE.of(principal);
            let repay_at = LEASE_START + Duration::YEAR + Duration::from_nanos(1);

            // LPP loan
            let loan = LoanResponse {
                principal_due: principal.into(),
                annual_interest_rate: LOAN_INTEREST_RATE,
                interest_paid: LEASE_START,
            };

            let mut loan = create_loan(loan);
            {
                let payment = one_year_margin + interest_payment;
                repay(
                    &mut loan,
                    payment,
                    state(principal, one_year_margin, one_year_interest, 0, 0),
                    receipt(principal, 0, one_year_margin, interest_payment, 0, 0, 0),
                    repay_at,
                );
            }
        }

        #[test]
        fn full_previous_partial_current_interest_repay() {
            let principal = 57326;
            let curr_interest_payment = 42;
            let one_year_margin = MARGIN_INTEREST_RATE.of(principal);
            let one_year_interest = LOAN_INTEREST_RATE.of(principal);

            // LPP loan
            let loan = LoanResponse {
                principal_due: principal.into(),
                annual_interest_rate: LOAN_INTEREST_RATE,
                interest_paid: LEASE_START,
            };

            let pay_since_start_current_period =
                Duration::YEAR - Duration::HOUR - Duration::HOUR - Duration::HOUR - Duration::HOUR;
            let repay_at = LEASE_START + Duration::YEAR + pay_since_start_current_period;
            let curr_margin_due =
                pay_since_start_current_period.annualized_slice_of(one_year_margin);
            let curr_interest_due =
                pay_since_start_current_period.annualized_slice_of(one_year_interest);
            let payment =
                one_year_margin + one_year_interest + curr_margin_due + curr_interest_payment;

            let mut loan = create_loan(loan);
            repay(
                &mut loan,
                payment,
                state(
                    principal,
                    one_year_margin,
                    one_year_interest,
                    curr_margin_due,
                    curr_interest_due,
                ),
                receipt(
                    principal,
                    0,
                    one_year_margin,
                    one_year_interest,
                    curr_margin_due,
                    curr_interest_payment,
                    0,
                ),
                repay_at,
            );
        }

        #[test]
        fn partial_principal_repay() {
            let principal = 36463892;
            let principal_paid = 234;
            let one_year_margin = MARGIN_INTEREST_RATE.of(principal);
            let one_year_interest = LOAN_INTEREST_RATE.of(principal);
            let duration_since_start = Duration::HOUR + Duration::HOUR + Duration::HOUR;
            let margin_due = duration_since_start.annualized_slice_of(one_year_margin);
            let interest_due = duration_since_start.annualized_slice_of(one_year_interest);
            let payment = margin_due + interest_due + principal_paid;

            let repay_at = LEASE_START + duration_since_start;
            let mut loan = create_loan(LoanResponse {
                principal_due: principal.into(),
                annual_interest_rate: LOAN_INTEREST_RATE,
                interest_paid: LEASE_START,
            });
            repay(
                &mut loan,
                payment,
                state(principal, 0, 0, margin_due, interest_due),
                receipt(principal, principal_paid, 0, 0, margin_due, interest_due, 0),
                repay_at,
            );
        }

        #[test]
        fn partial_interest_principal_repay() {
            let principal = 100;
            let principal_paid = 23;
            let one_year_margin = MARGIN_INTEREST_RATE.of(principal);
            let one_year_interest = LOAN_INTEREST_RATE.of(principal);
            let payment = one_year_margin + one_year_interest + principal_paid;

            let loan = LoanResponse {
                principal_due: principal.into(),
                annual_interest_rate: LOAN_INTEREST_RATE,
                interest_paid: LEASE_START,
            };

            let repay_at = LEASE_START + Duration::YEAR + Duration::from_nanos(1);
            let mut loan = create_loan(loan);
            repay(
                &mut loan,
                payment,
                state(principal, one_year_margin, one_year_interest, 0, 0),
                receipt(
                    principal,
                    principal_paid,
                    one_year_margin,
                    one_year_interest,
                    0,
                    0,
                    0,
                ),
                repay_at,
            );
        }

        #[test]
        fn full_principal_repay() {
            let principal = 3646389225881;
            let principal_paid = 234;
            let one_year_margin = MARGIN_INTEREST_RATE.of(principal);
            let one_year_interest = LOAN_INTEREST_RATE.of(principal);
            let duration_since_start = Duration::HOUR + Duration::HOUR + Duration::HOUR;
            let margin_due = duration_since_start.annualized_slice_of(one_year_margin);
            let interest_due = duration_since_start.annualized_slice_of(one_year_interest);
            let mut loan = create_loan(LoanResponse {
                principal_due: principal.into(),
                annual_interest_rate: LOAN_INTEREST_RATE,
                interest_paid: LEASE_START,
            });
            {
                let payment = margin_due + interest_due + principal_paid;
                let repay_at = LEASE_START + duration_since_start;
                repay(
                    &mut loan,
                    payment,
                    state(principal, 0, 0, margin_due, interest_due),
                    receipt(principal, principal_paid, 0, 0, margin_due, interest_due, 0),
                    repay_at,
                )
            }

            {
                let principal_due = principal - principal_paid;
                let change = 97;
                let duration_since_prev_payment = Duration::YEAR - duration_since_start;
                let curr_margin_due = duration_since_prev_payment
                    .annualized_slice_of(MARGIN_INTEREST_RATE.of(principal_due));
                let curr_interest_due = duration_since_prev_payment
                    .annualized_slice_of(LOAN_INTEREST_RATE.of(principal_due));
                let payment = curr_margin_due + curr_interest_due + principal_due + change;
                let repay_at = LEASE_START + Duration::YEAR;
                repay(
                    &mut loan,
                    payment,
                    state(principal_due, 0, 0, curr_margin_due, curr_interest_due),
                    receipt(
                        principal_due,
                        principal_due,
                        0,
                        0,
                        curr_margin_due,
                        curr_interest_due,
                        change,
                    ),
                    repay_at,
                )
            }
        }

        fn repay<P>(
            loan: &mut Loan<Lpn, LppLoanLocal>,
            payment: P,
            before_state: State<Lpn>,
            exp_receipt: RepayReceipt<Lpn>,
            now: Timestamp,
        ) where
            P: Into<Coin<Lpn>> + Copy,
        {
            let mut profit = profit_stub();

            assert_eq!(before_state, loan.state(now));

            let receipt = loan.repay(payment.into(), now, &mut profit).unwrap();

            assert_eq!(receipt, exp_receipt);
            assert_eq!(
                state(
                    before_state.principal_due - exp_receipt.principal_paid(),
                    before_state.previous_margin_interest_due - exp_receipt.previous_margin_paid(),
                    before_state.previous_interest_due - exp_receipt.previous_interest_paid(),
                    before_state.current_margin_interest_due - exp_receipt.current_margin_paid(),
                    before_state.current_interest_due - exp_receipt.current_interest_paid()
                ),
                loan.state(now)
            );
            assert_eq!(Into::<Batch>::into(profit), {
                let mut profit_amounts = vec![];
                if exp_receipt.previous_margin_paid() != Coin::default() {
                    profit_amounts.push(exp_receipt.previous_margin_paid());
                }
                if exp_receipt.current_margin_paid() != Coin::default() {
                    profit_amounts.push(exp_receipt.current_margin_paid());
                }
                if !profit_amounts.is_empty() {
                    bank::bank_send_multiple(Batch::default(), PROFIT_ADDR, &profit_amounts)
                } else {
                    Batch::default()
                }
            })
        }

        fn state<P>(
            principal: P,
            previous_margin_interest_due: P,
            previous_interest_due: P,
            current_margin_interest_due: P,
            current_interest_due: P,
        ) -> State<Lpn>
        where
            P: Into<Coin<Lpn>>,
        {
            State {
                annual_interest: LOAN_INTEREST_RATE,
                annual_interest_margin: MARGIN_INTEREST_RATE,
                principal_due: principal.into(),
                previous_margin_interest_due: previous_margin_interest_due.into(),
                previous_interest_due: previous_interest_due.into(),
                current_margin_interest_due: current_margin_interest_due.into(),
                current_interest_due: current_interest_due.into(),
            }
        }

        fn receipt<P>(
            principal: P,
            paid_principal: P,
            paid_previous_margin_interest_due: P,
            paid_previous_interest_due: P,
            paid_current_margin_interest_due: P,
            paid_current_interest_due: P,
            change: P,
        ) -> RepayReceipt<Lpn>
        where
            P: Into<Coin<Lpn>>,
        {
            let mut receipt = RepayReceipt::default();
            receipt.pay_principal(principal.into(), paid_principal.into());
            receipt.pay_previous_margin(paid_previous_margin_interest_due.into());
            receipt.pay_previous_interest(paid_previous_interest_due.into());
            receipt.pay_current_margin(paid_current_margin_interest_due.into());
            receipt.pay_current_interest(paid_current_interest_due.into());
            receipt.keep_change(change.into());
            receipt
        }
    }

    #[cfg(test)]
    mod test_state {
        use cosmwasm_std::Timestamp;
        use finance::{
            coin::Coin, currency::Currency, duration::Duration, interest::InterestPeriod,
            percent::Percent,
        };
        use lpp::msg::LoanResponse;

        use crate::loan::tests::create_loan;

        use super::{LEASE_START, MARGIN_INTEREST_RATE};

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
                    if now <= LEASE_START + Duration::YEAR {
                        Duration::default()
                    } else {
                        Duration::between(paid, LEASE_START + Duration::YEAR)
                    },
                    principal_due,
                    rate,
                ),
                interest(
                    Duration::between(
                        if now <= LEASE_START + Duration::YEAR {
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
            let principal_due = 10000.into();

            let interest_rate = Percent::from_permille(25);

            let loan_resp = LoanResponse {
                principal_due,
                annual_interest_rate: interest_rate,
                interest_paid: LEASE_START,
            };

            let now = LEASE_START + period;
            let loan = create_loan(loan_resp.clone());

            let (expected_margin_overdue, expected_margin_due) =
                margin_interests(loan_resp.interest_paid, now, principal_due);

            let (expected_interest_overdue, expected_interest_due) = interests(
                loan_resp.interest_paid,
                now,
                principal_due,
                loan_resp.annual_interest_rate,
            );

            let res = loan.state(now);

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
    }

    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    struct LppLoanLocal {
        loan: LoanResponse<Lpn>,
    }

    impl LppLoanLocal {
        fn new(loan: LoanResponse<Lpn>) -> Self {
            Self { loan }
        }
    }

    impl LppLoanTrait<Lpn> for LppLoanLocal {
        fn principal_due(&self) -> Coin<Lpn> {
            self.loan.principal_due
        }

        fn interest_due(&self, by: Timestamp) -> Coin<Lpn> {
            self.loan.interest_due(by)
        }

        fn repay(&mut self, by: Timestamp, repayment: Coin<Lpn>) -> LppResult<()> {
            self.loan.repay(by, repayment).map(|_| ())
        }

        fn annual_interest_rate(&self) -> Percent {
            self.loan.annual_interest_rate
        }
    }

    impl From<LppLoanLocal> for LppBatch<LppRef> {
        fn from(_: LppLoanLocal) -> Self {
            unreachable!()
        }
    }

    fn create_loan(loan: LoanResponse<Lpn>) -> Loan<Lpn, LppLoanLocal> {
        Loan::new(
            LEASE_START,
            LppLoanLocal::new(loan),
            MARGIN_INTEREST_RATE,
            InterestPaymentSpec::new(Duration::YEAR, Duration::from_secs(0)),
        )
    }

    fn profit_stub() -> ProfitStub<LazySenderStub> {
        ProfitRef::unchecked(PROFIT_ADDR).as_stub()
    }
}
