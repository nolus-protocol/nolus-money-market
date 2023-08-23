use std::{fmt::Debug, marker::PhantomData};

use serde::{Deserialize, Serialize};

use currency::Currency;
use finance::{
    coin::Coin,
    interest::InterestPeriod,
    percent::{Percent, Units},
    period::Period,
    zero::Zero,
};
use lpp::{
    loan::RepayShares,
    stub::{loan::LppLoan as LppLoanTrait, LppBatch, LppRef},
};
use platform::{bank::FixedAddressSender, batch::Batch};
use profit::stub::ProfitRef;
use sdk::cosmwasm_std::Timestamp;

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
    lpp: LppRef,
    interest_payment_spec: InterestPaymentSpec,
    current_period: InterestPeriod<Units, Percent>,
    profit: ProfitRef,
}

impl LoanDTO {
    fn new(
        lpp: LppRef,
        interest_payment_spec: InterestPaymentSpec,
        current_period: InterestPeriod<Units, Percent>,
        profit: ProfitRef,
    ) -> Self {
        Self {
            lpp,
            interest_payment_spec,
            current_period,
            profit,
        }
    }

    pub(crate) fn annual_margin_interest(&self) -> Percent {
        self.current_period.interest_rate()
    }

    pub(crate) fn lpp(&self) -> &LppRef {
        &self.lpp
    }

    pub(crate) fn profit(&self) -> &ProfitRef {
        &self.profit
    }
}

pub struct Loan<Lpn, LppLoan> {
    lpn: PhantomData<Lpn>,
    lpp_loan: LppLoan,
    interest_payment_spec: InterestPaymentSpec,
    due_period: InterestPeriod<Units, Percent>,
}

impl<Lpn, LppLoan> Loan<Lpn, LppLoan>
where
    Lpn: Currency + Debug,
    LppLoan: LppLoanTrait<Lpn>,
    LppLoan::Error: Into<ContractError>,
{
    pub(super) fn try_into_dto(self, profit: ProfitRef) -> ContractResult<(LoanDTO, Batch)> {
        Self::try_loan_into(self.lpp_loan).map(|lpp_batch: LppBatch<LppRef>| {
            (
                LoanDTO::new(
                    lpp_batch.lpp_ref,
                    self.interest_payment_spec,
                    self.due_period,
                    profit,
                ),
                lpp_batch.batch,
            )
        })
    }

    pub(super) fn try_into_messages(self) -> ContractResult<Batch> {
        Self::try_loan_into(self.lpp_loan).map(|lpp_batch: LppBatch<LppRef>| lpp_batch.batch)
    }

    fn try_loan_into(loan: LppLoan) -> ContractResult<LppBatch<LppRef>> {
        loan.try_into().map_err(Into::<ContractError>::into)
    }
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
        let due_period = InterestPeriod::with_interest(annual_margin_interest).and_period(
            Period::from_length(start, interest_payment_spec.due_period()),
        );
        Self {
            lpn: PhantomData,
            lpp_loan,
            interest_payment_spec,
            due_period,
        }
    }

    pub(super) fn from_dto(dto: LoanDTO, lpp_loan: LppLoan) -> Self {
        {
            Self {
                lpn: PhantomData,
                lpp_loan,
                interest_payment_spec: dto.interest_payment_spec,
                due_period: dto.current_period,
            }
        }
    }

    pub(crate) fn grace_period_end(&self) -> Timestamp {
        self.grace_period_end_impl(&self.due_period.period())
    }

    pub(crate) fn grace_period_end_not_before(&self, when: &Timestamp) -> Timestamp {
        let mut current_period = self.due_period.period();
        while &self.grace_period_end_impl(&current_period) < when {
            current_period = next_due_period(current_period, &self.interest_payment_spec);
        }
        self.grace_period_end_impl(&current_period)
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
        Profit: FixedAddressSender,
    {
        self.debug_check_start_due_before(by, "before the 'repay-by' time");

        let prev_period_receipt = self.repay_prev_periods(payment, by);
        profit.send(prev_period_receipt.margin_paid);
        let change = prev_period_receipt.change;

        let mut receipt = RepayReceipt::default();
        receipt.pay_previous_margin(prev_period_receipt.margin_paid);
        receipt.pay_previous_interest(prev_period_receipt.interest_paid);
        debug_assert_eq!(payment, change + receipt.total());

        let change = if !self.overdue_at(by) {
            let period_receipt = self.repay_due_period(change, by);
            profit.send(period_receipt.margin_paid);
            receipt.pay_current_margin(period_receipt.margin_paid);
            receipt.pay_current_interest(period_receipt.interest_paid);

            let principal_due = self.lpp_loan.principal_due();
            let (principal_paid, change) = self.repay_principal(period_receipt.change, by);
            receipt.pay_principal(principal_due, principal_paid);
            change
        } else {
            debug_assert!(change == Coin::ZERO);
            change
        };
        debug_assert_eq!(payment, change + receipt.total());

        receipt.keep_change(change);
        debug_assert_eq!(payment, receipt.total());
        Ok(receipt)
    }

    pub(crate) fn state(&self, now: Timestamp) -> State<Lpn> {
        self.debug_check_start_due_before(now, "in the past. Now is ");

        let principal_due = self.lpp_loan.principal_due();

        let mut current_period = self.due_period_before_payments();
        while overdue_at(&current_period, now) {
            current_period = next_due_period(current_period, &self.interest_payment_spec);
        }
        let current_due_period = self.due_period.and_period(Period::from_till(
            current_period.start().max(self.due_period.start()),
            now,
        ));

        let prev_due_periods = self.due_period.and_period(Period::from_till(
            current_period.start().min(self.due_period.start()),
            current_period.start(),
        ));

        let previous_margin_interest_due = prev_due_periods.interest(principal_due);
        let current_margin_interest_due = current_due_period.interest(principal_due);

        let previous_interest_due = self.lpp_loan.interest_due(prev_due_periods.till());
        let current_interest_due = self.lpp_loan.interest_due(now) - previous_interest_due;

        State {
            annual_interest: self.lpp_loan.annual_interest_rate(),
            annual_interest_margin: self.due_period.interest_rate(),
            principal_due,
            previous_interest_due,
            current_interest_due,
            previous_margin_interest_due,
            current_margin_interest_due,
        }
    }

    fn grace_period_end_impl(&self, due_period: &Period) -> Timestamp {
        due_period.till() + self.interest_payment_spec.grace_period()
    }

    fn repay_prev_periods(&mut self, payment: Coin<Lpn>, by: Timestamp) -> RepayPeriodReceipt<Lpn> {
        let mut margin_paid = Coin::default();
        let mut interest_paid = Coin::default();
        let mut change = payment;
        while self.overdue_at(by) && change != Coin::ZERO {
            let period_receipt = self.repay_due_period(change, self.due_period.till());
            margin_paid += period_receipt.margin_paid;
            interest_paid += period_receipt.interest_paid;
            change = period_receipt.change;
        }
        RepayPeriodReceipt::with_margin(margin_paid).and_interest(interest_paid, change)
    }

    fn repay_due_period(&mut self, payment: Coin<Lpn>, by: Timestamp) -> RepayPeriodReceipt<Lpn> {
        self.debug_check_late_payment(by, "due period");
        let (prev_margin_paid, change) =
            self.repay_margin_interest(self.lpp_loan.principal_due(), by, payment);
        let res = RepayPeriodReceipt::with_margin(prev_margin_paid);

        if change.is_zero() {
            return res;
        }
        debug_assert_eq!(
            Coin::ZERO,
            self.due_period
                .and_period(Period::from_till(self.due_period.start(), by))
                .interest(self.lpp_loan.principal_due()),
            "some margin left"
        );

        // In rare cases there still might be some very tiny due period for which the calculated
        // due amount is zero. Therefore, we should pay the loan interest up to that time
        // to avoid breaking the invariant 'loan_interest.paid_by_time <= margin_interest.paid_by_time'.
        self.repay_loan_interest(change, self.due_period.start(), res)
    }

    fn repay_margin_interest(
        &mut self,
        principal_due: Coin<Lpn>,
        by: Timestamp,
        payment: Coin<Lpn>,
    ) -> (Coin<Lpn>, Coin<Lpn>) {
        self.debug_check_late_payment(by, "margin interest");
        let (period, change) = self.due_period.pay(principal_due, payment, by);
        self.due_period = period;

        (payment - change, change)
    }

    fn repay_loan_interest(
        &mut self,
        payment: Coin<Lpn>,
        by: Timestamp,
        receipt: RepayPeriodReceipt<Lpn>,
    ) -> RepayPeriodReceipt<Lpn> {
        self.debug_check_late_payment(by, "loan interest");
        let due = self.lpp_loan.interest_due(by);
        let paid = due.min(payment);
        let change = payment - paid;

        let repay_shares = self.lpp_loan.repay(by, paid);
        debug_assert_eq!(
            RepayShares {
                interest: paid,
                ..Default::default()
            },
            repay_shares
        );

        if paid == due && by == self.due_period.till() {
            self.open_next_period();
        }
        receipt.and_interest(paid, change)
    }

    fn repay_principal(&mut self, payment: Coin<Lpn>, by: Timestamp) -> (Coin<Lpn>, Coin<Lpn>) {
        self.debug_check_late_payment(by, "principal");
        let paid = payment.min(self.lpp_loan.principal_due());
        let repay_shares = self.lpp_loan.repay(by, paid);
        debug_assert_eq!(
            RepayShares {
                principal: paid,
                ..Default::default()
            },
            repay_shares
        );
        (paid, payment - paid)
    }

    fn open_next_period(&mut self) {
        debug_assert!(self.due_period.zero_length());

        self.due_period = self.due_period.and_period(next_due_period(
            self.due_period.period(),
            &self.interest_payment_spec,
        ));
    }

    fn overdue_at(&self, when: Timestamp) -> bool {
        overdue_at(&self.due_period.period(), when)
    }

    fn due_period_before_payments(&self) -> Period {
        self.due_period
            .period()
            .this(self.interest_payment_spec.due_period())
    }

    fn debug_check_start_due_before(&self, when: Timestamp, when_descr: &str) {
        debug_assert!(
            self.due_period.start() <= when,
            "The current due period starting at {}s, should begin {} {}s",
            self.due_period.start(),
            when_descr,
            when
        );
    }

    #[track_caller]
    fn debug_check_late_payment(&self, when: Timestamp, what_descr: &str) {
        debug_assert!(
            !self.overdue_at(when),
            "An attempt to repay {what_descr} at {when} that is past the due period end time {end_time}!",
            end_time = self.due_period.till(),
        );
    }
}

fn overdue_at(due_period: &Period, when: Timestamp) -> bool {
    due_period.till() < when
}

fn next_due_period(due_period: Period, payment_spec: &InterestPaymentSpec) -> Period {
    due_period.next(payment_spec.due_period())
}

struct RepayPeriodReceipt<C>
where
    C: Currency,
{
    margin_paid: Coin<C>,
    interest_paid: Coin<C>,
    change: Coin<C>,
}
impl<C> RepayPeriodReceipt<C>
where
    C: Currency,
{
    fn with_margin(margin_paid: Coin<C>) -> Self {
        Self {
            margin_paid,
            interest_paid: Coin::default(),
            change: Coin::default(),
        }
    }

    fn and_interest(self, interest_paid: Coin<C>, change: Coin<C>) -> Self {
        Self {
            margin_paid: self.margin_paid,
            interest_paid,
            change,
        }
    }
}

#[cfg(test)]
mod tests {
    use serde::{Deserialize, Serialize};

    use currency::lpn::Usdc;
    use finance::{coin::Coin, duration::Duration, percent::Percent};
    use lpp::{
        error::ContractError as LppError,
        loan::RepayShares,
        msg::LoanResponse,
        stub::{loan::LppLoan as LppLoanTrait, LppBatch, LppRef},
    };
    use platform::bank::FixedAddressSender;
    use profit::stub::ProfitRef;
    use sdk::cosmwasm_std::Timestamp;

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

        use currency::{Currency, Group};
        use finance::{
            coin::{Amount, Coin, WithCoin},
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
        fn multiple_periods() {
            let principal = 1000;
            let interest_payment = 43;
            let one_year_margin = MARGIN_INTEREST_RATE.of(principal);
            let one_year_interest = LOAN_INTEREST_RATE.of(principal);
            let since_start_current_period = Duration::from_days(120);
            let repay_at = LEASE_START
                + Duration::YEAR
                + Duration::YEAR
                + Duration::YEAR
                + since_start_current_period;

            let curr_margin_due = since_start_current_period.annualized_slice_of(one_year_margin);
            let curr_margin_paid = curr_margin_due / 2;
            let curr_interest_due =
                since_start_current_period.annualized_slice_of(one_year_interest);

            let loan = LoanResponse {
                principal_due: principal.into(),
                annual_interest_rate: LOAN_INTEREST_RATE,
                interest_paid: LEASE_START,
            };

            let mut loan = create_loan(loan);
            {
                let payment =
                    one_year_margin + one_year_interest + one_year_margin + interest_payment;
                repay(
                    &mut loan,
                    payment,
                    state(
                        principal,
                        one_year_margin * 3,
                        one_year_interest * 3,
                        curr_margin_due,
                        curr_interest_due,
                    ),
                    receipt(
                        principal,
                        0,
                        one_year_margin * 2,
                        one_year_interest + interest_payment,
                        0,
                        0,
                        0,
                    ),
                    repay_at,
                );
            }
            {
                let payment = interest_payment;
                repay(
                    &mut loan,
                    payment,
                    state(
                        principal,
                        one_year_margin,
                        one_year_interest * 2 - interest_payment,
                        curr_margin_due,
                        curr_interest_due,
                    ),
                    receipt(principal, 0, 0, interest_payment, 0, 0, 0),
                    repay_at,
                );
            }
            {
                let payment = one_year_margin + one_year_interest * 2 - interest_payment * 2
                    + curr_margin_paid;
                repay(
                    &mut loan,
                    payment,
                    state(
                        principal,
                        one_year_margin,
                        one_year_interest * 2 - interest_payment * 2,
                        curr_margin_due,
                        curr_interest_due,
                    ),
                    receipt(
                        principal,
                        0,
                        one_year_margin,
                        one_year_interest * 2 - interest_payment * 2,
                        curr_margin_paid,
                        0,
                        0,
                    ),
                    repay_at,
                );
            }
            {
                let change = 3;
                let payment =
                    (curr_margin_due - curr_margin_paid) + curr_interest_due + principal + change;
                repay(
                    &mut loan,
                    payment,
                    state(
                        principal,
                        0,
                        0,
                        curr_margin_due - curr_margin_paid,
                        curr_interest_due,
                    ),
                    receipt(
                        principal,
                        principal,
                        0,
                        0,
                        curr_margin_due - curr_margin_paid,
                        curr_interest_due,
                        change,
                    ),
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

            let since_start_current_period =
                Duration::YEAR - Duration::HOUR - Duration::HOUR - Duration::HOUR - Duration::HOUR;
            let repay_at = LEASE_START + Duration::YEAR + since_start_current_period;
            let curr_margin_due = since_start_current_period.annualized_slice_of(one_year_margin);
            let curr_interest_due =
                since_start_current_period.annualized_slice_of(one_year_interest);
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

            assert_eq!(exp_receipt, receipt);
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
            assert_eq!(
                {
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
                },
                Into::<Batch>::into(profit)
            )
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

    mod test_state {
        use currency::Currency;
        use finance::{
            coin::Coin, duration::Duration, interest::InterestPeriod, percent::Percent,
            period::Period,
        };
        use lpp::msg::LoanResponse;
        use sdk::cosmwasm_std::Timestamp;

        use crate::loan::tests::create_loan;

        use super::{LEASE_START, MARGIN_INTEREST_RATE};

        fn interest<Lpn>(period_len: Duration, principal_due: Coin<Lpn>, rate: Percent) -> Coin<Lpn>
        where
            Lpn: Currency,
        {
            InterestPeriod::with_interest(rate)
                .and_period(Period::from_length(Timestamp::default(), period_len))
                .interest(principal_due)
        }

        fn interests<Lpn>(
            paid_by: Timestamp,
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
                        Duration::between(paid_by, LEASE_START + Duration::YEAR)
                    },
                    principal_due,
                    rate,
                ),
                interest(
                    Duration::between(
                        if now <= LEASE_START + Duration::YEAR {
                            paid_by
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
            paid_by: Timestamp,
            now: Timestamp,
            principal_due: Coin<Lpn>,
        ) -> (Coin<Lpn>, Coin<Lpn>)
        where
            Lpn: Currency,
        {
            interests(paid_by, now, principal_due, MARGIN_INTEREST_RATE)
        }

        fn test_state(period: Duration) {
            let principal_due = 10000.into();

            let loan_interest_rate = Percent::from_permille(25);

            let loan_resp = LoanResponse {
                principal_due,
                annual_interest_rate: loan_interest_rate,
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
                expected_margin_overdue, res.previous_margin_interest_due,
                "Got different margin overdue than expected!",
            );

            assert_eq!(
                expected_margin_due, res.current_margin_interest_due,
                "Got different margin due than expected!",
            );

            assert_eq!(
                expected_interest_overdue, res.previous_interest_due,
                "Got different interest overdue than expected!",
            );

            assert_eq!(
                expected_interest_due, res.current_interest_due,
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

    mod test_grace_period_end {
        use finance::duration::Duration;
        use lpp::msg::LoanResponse;

        use crate::api::InterestPaymentSpec;

        use super::{create_loan_with_interest_spec, LEASE_START, LOAN_INTEREST_RATE};

        #[test]
        fn in_current_period() {
            const BIT: Duration = Duration::from_nanos(1);
            let due_period = Duration::YEAR;
            let grace_period = Duration::HOUR;
            let next_grace_period_end = LEASE_START + due_period + grace_period;

            let loan = create_loan_with_interest_spec(
                LoanResponse {
                    principal_due: 1000.into(),
                    annual_interest_rate: LOAN_INTEREST_RATE,
                    interest_paid: LEASE_START,
                },
                InterestPaymentSpec::new(due_period, grace_period),
            );
            assert_eq!(next_grace_period_end, loan.grace_period_end());
            assert_eq!(
                next_grace_period_end,
                loan.grace_period_end_not_before(&(LEASE_START + Duration::from_days(10)))
            );
            assert_eq!(
                next_grace_period_end,
                loan.grace_period_end_not_before(&(LEASE_START + due_period))
            );
            assert_eq!(
                next_grace_period_end,
                loan.grace_period_end_not_before(&(LEASE_START + due_period + BIT))
            );
            assert_eq!(
                next_grace_period_end,
                loan.grace_period_end_not_before(&(next_grace_period_end - BIT))
            );
            assert_eq!(
                next_grace_period_end,
                loan.grace_period_end_not_before(&next_grace_period_end)
            );
            assert_eq!(
                next_grace_period_end + due_period,
                loan.grace_period_end_not_before(&(next_grace_period_end + BIT))
            );
            assert_eq!(
                next_grace_period_end + due_period,
                loan.grace_period_end_not_before(&(next_grace_period_end + due_period - BIT))
            );
            assert_eq!(
                next_grace_period_end + due_period,
                loan.grace_period_end_not_before(&(next_grace_period_end + due_period))
            );
            assert_eq!(
                next_grace_period_end + due_period + due_period,
                loan.grace_period_end_not_before(&(next_grace_period_end + due_period + BIT))
            );
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

        fn repay(&mut self, by: Timestamp, repayment: Coin<Lpn>) -> RepayShares<Lpn> {
            self.loan.repay(by, repayment)
        }

        fn annual_interest_rate(&self) -> Percent {
            self.loan.annual_interest_rate
        }
    }

    impl TryFrom<LppLoanLocal> for LppBatch<LppRef> {
        type Error = LppError;
        fn try_from(_: LppLoanLocal) -> LppResult<Self> {
            unreachable!()
        }
    }

    fn create_loan(loan: LoanResponse<Lpn>) -> Loan<Lpn, LppLoanLocal> {
        create_loan_with_interest_spec(
            loan,
            InterestPaymentSpec::new(Duration::YEAR, Duration::from_secs(0)),
        )
    }
    fn create_loan_with_interest_spec(
        loan: LoanResponse<Lpn>,
        interest_spec: InterestPaymentSpec,
    ) -> Loan<Lpn, LppLoanLocal> {
        Loan::new(
            LEASE_START,
            LppLoanLocal::new(loan),
            MARGIN_INTEREST_RATE,
            interest_spec,
        )
    }

    fn profit_stub() -> impl FixedAddressSender {
        ProfitRef::unchecked(PROFIT_ADDR).as_stub()
    }
}
