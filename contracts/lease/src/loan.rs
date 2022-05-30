use std::fmt::Debug;

use cosmwasm_std::{Addr, Coin, QuerierWrapper, SubMsg, Timestamp};
use finance::{coin, duration::Duration, interest::InterestPeriod, percent::Percent};
use lpp::{msg::QueryLoanResponse, stub::Lpp};
use serde::{Deserialize, Serialize};

use crate::error::{ContractError, ContractResult};

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "snake_case")]
/// The value remains intact.
pub struct Loan<L> {
    annual_margin_interest: Percent,
    lpp: L,
    interest_due_period_secs: u32,
    grace_period_secs: u32,
    current_period: InterestPeriod,
}

impl<L> Loan<L>
where
    L: Lpp,
{
    pub(crate) fn open(
        when: Timestamp,
        lpp: L,
        annual_margin_interest: Percent,
        interest_due_period_secs: u32,
        grace_period_secs: u32,
    ) -> ContractResult<Self> {
        // check them out cw_utils::Duration, cw_utils::NativeBalance
        Ok(Self {
            annual_margin_interest,
            lpp,
            interest_due_period_secs,
            grace_period_secs,
            current_period: InterestPeriod::with_interest(annual_margin_interest)
                .from(when)
                .spanning(Duration::from_secs(interest_due_period_secs)),
        })
    }

    pub(crate) fn closed(&self, querier: &QuerierWrapper, lease: Addr) -> ContractResult<bool> {
        // TODO define lpp::Loan{querier, id = lease_id: Addr} and instantiate it on Lease::load
        self.lpp
            .loan(querier, lease)
            .map(|res| res.is_none())
            .map_err(|err| err.into())
    }

    pub(crate) fn repay(
        &mut self,
        payment: Coin,
        by: Timestamp,
        querier: &QuerierWrapper,
        lease: Addr,
    ) -> ContractResult<Option<SubMsg>> {
        let principal_due = self.load_principal_due(querier, lease.clone())?;
        debug_assert_eq!(payment.denom, principal_due.denom);

        let change = self.repay_margin_interest(&principal_due, by, payment);
        if change.amount.is_zero() {
            return Ok(None);
        }

        let loan_interest_due =
            self.load_loan_interest_due(querier, lease, self.current_period.start())?;
        debug_assert_eq!(change.denom, loan_interest_due.denom);

        let loan_payment =
            if loan_interest_due.amount <= change.amount && self.current_period.zero_length() {
                self.open_next_period();
                let loan_interest_surplus = coin::sub_amount(change, loan_interest_due.amount);
                let change = self.repay_margin_interest(&principal_due, by, loan_interest_surplus);
                coin::add_coin(loan_interest_due, change)
            } else {
                change
            };
        if loan_payment.amount.is_zero() {
            // in practice not possible, but in theory it is if two consecutive repayments are received
            // with the same 'by' time.
            return Ok(None);
        }
        // TODO handle any surplus left after the repayment, options:
        // - query again the lpp on the interest due by now + calculate the max repayment by now + send the supplus to the customer, or
        // - [better separation of responsabilities, need of a 'reply' contract entry] pay lpp and once the surplus is received send it to the customer, or
        // - [better separation of responsabilities + low trx cost] keep the surplus in the lease and send it back on lease.close
        // - [better separation of responsabilities + even lower trx cost] include the remaining interest due up to this moment in the Lpp.query_loan response
        // and send repayment amount up to the principal + interest due. The remainder is left in the lease

        // TODO For repayment, use not only the amount received but also the amount present in the lease. The latter may have been left as a surplus from a previous payment.
        self.lpp
            .repay_loan_req(loan_payment)
            .map(Some)
            .map_err(|err| err.into())
    }

    fn load_principal_due(
        &self,
        querier: &QuerierWrapper,
        lease: impl Into<Addr>,
    ) -> ContractResult<Coin> {
        let loan: QueryLoanResponse = self.lpp.loan(querier, lease).map_err(ContractError::from)?;
        Ok(loan.ok_or(ContractError::LoanClosed())?.principal_due)
    }

    fn load_loan_interest_due(
        &self,
        querier: &QuerierWrapper,
        lease: impl Into<Addr>,
        by: Timestamp,
    ) -> ContractResult<Coin> {
        let interest = self
            .lpp
            .loan_outstanding_interest(querier, lease, by)
            .map_err(ContractError::from)?;
        Ok(interest.ok_or(ContractError::LoanClosed())?.0)
    }

    fn repay_margin_interest(
        &mut self,
        principal_due: &Coin,
        by: Timestamp,
        payment: Coin,
    ) -> Coin {
        let (period, change) = self.current_period.pay(principal_due, payment, by);
        self.current_period = period;
        change
    }

    fn open_next_period(&mut self) {
        debug_assert!(self.current_period.zero_length());

        self.current_period = InterestPeriod::with_interest(self.annual_margin_interest)
            .from(self.current_period.till())
            .spanning(Duration::from_secs(self.interest_due_period_secs));
    }
}
