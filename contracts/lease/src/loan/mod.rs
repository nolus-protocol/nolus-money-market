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

pub(crate) use repay::{Result as RepayResult, LoanInterestsPaid};

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
    Lpn: Currency,
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
                batch: Batch::from(self),
                paid,
            })
    }

    fn repay_inner(
        &mut self,
        payment: Coin<Lpn>,
        by: Timestamp,
        lease: Addr,
    ) -> ContractResult<LoanInterestsPaid<Lpn>> {
        self.debug_check_start_due_before(by, "before the 'repay-by' time");

        let (principal_due, mut interest_due) = self.load_lpp_loan(lease.clone())?
            .ok_or(ContractError::LoanClosed())
            .map(|resp| (resp.principal_due, resp.interest_due))?;

        let mut paid = LoanInterestsPaid::default();

        let change = self.repay_margin_interest(principal_due, by, payment, &mut paid);

        if change.is_zero() {
            return Ok(paid);
        }

        let interest_overdue = self.load_loan_interest_due(lease, self.current_period.start())?;

        interest_due -= interest_overdue;

        let loan_payment = if interest_overdue <= change && self.current_period.zero_length() {
            paid.pay_next_interest(interest_overdue);
            self.open_next_period();

            let surplus = change - interest_overdue;

            interest_overdue + self.repay_margin_interest(principal_due, by, surplus, &mut paid)
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
        //  - query again the lpp on the interest due by now + calculate the max repayment by now + send the supplus to the customer, or
        //  - [better separation of responsabilities, need of a 'reply' contract entry] pay lpp and once the surplus is received send it to the customer, or
        //  - [better separation of responsabilities + low trx cost] keep the surplus in the lease and send it back on lease.close
        //  - [better separation of responsabilities + even lower trx cost] include the remaining interest due up to this moment in the Lpp.query_loan response
        //  and send repayment amount up to the principal + interest due. The remainder is left in the lease

        // TODO For repayment, use not only the amount received but also the amount present in the lease. The latter may have been left as a surplus from a previous payment.
        self.lpp.repay_loan_req(loan_payment)?;

        paid.pay_next_interest(interest_due - interest_overdue);

        paid.pay_principal(principal_due, loan_payment - interest_due);

        assert!(
            paid.previous_margin_paid() + paid.current_margin_paid() +
            paid.previous_interest_paid() + paid.current_interest_paid() +
            paid.principal_paid() == payment,
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
        paid: &mut LoanInterestsPaid<Lpn>,
    ) -> Coin<Lpn> {
        let (period, change) = self.current_period.pay(principal_due, payment, by);
        self.current_period = period;

        paid.pay_next_margin(payment - change);

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
