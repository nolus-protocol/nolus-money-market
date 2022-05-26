use std::fmt::Debug;

use cosmwasm_std::{Addr, Coin, QuerierWrapper, SubMsg, Timestamp};
use finance::{coin, duration::Duration, interest::InterestPeriod, percent::Percent};
use lpp::stub::Lpp;
use serde::{Deserialize, Serialize};

use crate::error::ContractResult;

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
            .loan_closed(querier, lease)
            .map_err(|err| err.into())
    }

    pub(crate) fn repay(&mut self, payment: Coin, by: Timestamp) -> ContractResult<Option<SubMsg>> {
        // TODO self.lpp.my_loan()
        let principal_due: Coin = Coin::new(10, &payment.denom);
        let change = self.repay_margin_interest(&principal_due, by, payment);
        if change.amount.is_zero() {
            return Ok(None);
        }

        // TODO self.lpp.my_interest_due(by = self.current_period().till(): Timestamp)
        let loan_interest_due = Coin::new(1000, &principal_due.denom);
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

        self.lpp
            .repay_loan_req(loan_payment)
            .map(Some)
            .map_err(|err| err.into())
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
