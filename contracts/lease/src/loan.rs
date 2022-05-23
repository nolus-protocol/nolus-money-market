use std::fmt::Debug;

use cosmwasm_std::{Coin, Timestamp, SubMsg};
use finance::{interest::InterestPeriod, duration::Duration, coin, percent::Percent};
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

    pub(crate) fn repay(&mut self, payment: Coin, by: Timestamp) -> ContractResult<Option<SubMsg>> {
        // TODO self.lpp.my_loan()
        let principal_due: Coin = Coin::new(10, &payment.denom);
        let (period, change) = self.current_period.pay(&principal_due, payment, by);
        self.current_period = period;
        // TODO self.lpp.my_interest_due(by: Timestamp)
        let loan_interest_due = Coin::new(1000, &principal_due.denom);
        let _loan_payment = if loan_interest_due.amount <= change.amount && self.current_period.zero_length() {
            self.current_period = InterestPeriod::with_interest(self.annual_margin_interest)
                .from(self.current_period.till())
                .spanning(Duration::from_secs(self.interest_due_period_secs));
            let (period, change) =
                self.current_period
                    .pay(&principal_due, coin::sub_amount(change, loan_interest_due.amount), by);
            self.current_period = period;
            coin::add_coin(loan_interest_due, change)
        } else {
            change
        };
        // TODO self.lpp.repay_loan_req(&self, repayment: Coin)
        Ok(None)
    }
}