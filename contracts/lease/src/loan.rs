use std::{cmp, fmt::Debug};

use cosmwasm_std::{Coin, Timestamp, Uint128, SubMsg};
use finance::percent::Percent;
use lpp::stub::Lpp;
use serde::{Deserialize, Serialize};

use crate::{duration::Duration, error::ContractResult};

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "snake_case")]
/// The value remains intact.
pub struct Loan<L> {
    annual_margin_interest_permille: u8,
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
        annual_margin_interest_permille: u8,
        interest_due_period_secs: u32,
        grace_period_secs: u32,
    ) -> ContractResult<Self> {
        // check them out cw_utils::Duration, cw_utils::NativeBalance
        Ok(Self {
            annual_margin_interest_permille,
            lpp,
            interest_due_period_secs,
            grace_period_secs,
            current_period: InterestPeriod::new(annual_margin_interest_permille)
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
            self.current_period = InterestPeriod::new(self.annual_margin_interest_permille)
                .from(self.current_period.till())
                .spanning(Duration::from_secs(self.interest_due_period_secs));
            let (period, change) =
                self.current_period
                    .pay(&principal_due, sub_amount(change, loan_interest_due.amount), by);
            self.current_period = period;
            add_coin(loan_interest_due, change)
        } else {
            change
        };
        // TODO self.lpp.repay_loan_req(&self, repayment: Coin)
        Ok(None)
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
struct InterestPeriod {
    start: Timestamp,
    length: Duration,
    interest: u8, //TODO define Permille
}

impl InterestPeriod {
    pub fn new(interest: u8) -> Self {
        Self {
            start: Timestamp::default(),
            length: Duration::default(),
            interest,
        }
    }

    pub fn from(self, start: Timestamp) -> Self {
        Self {
            start,
            length: self.length,
            interest: self.interest,
        }
    }

    pub fn spanning(self, length: Duration) -> Self {
        Self {
            start: self.start,
            length,
            interest: self.interest,
        }
    }

    pub fn shift_start(self, delta: Duration) -> Self {
        assert!(delta <= self.length);
        let res = Self {
            start: self.start + delta,
            length: self.length - delta,
            interest: self.interest,
        };
        debug_assert!(self.till() == res.till());
        res
    }

    pub fn zero_length(&self) -> bool {
        self.length == Duration::default()
    }

    pub fn till(&self) -> Timestamp {
        self.start + self.length
    }

    pub fn pay(self, principal: &Coin, payment: Coin, by: Timestamp) -> (Self, Coin) {
        let till = cmp::min(cmp::max(self.start, by), self.till());
        debug_assert!(self.start <= till);
        debug_assert!(till <= self.till());
        let period = Duration::between(self.start, till);

        let interest_due_per_year = Percent::from(self.interest).of(principal);
        let interest_due_per_period =
            fraction(interest_due_per_year.amount, period, Duration::YEAR);

        let repayment = cmp::min(interest_due_per_period, payment.amount);
        let period_paid_for = fraction(period, repayment, interest_due_per_period);

        let change = sub_amount(payment, repayment);
        (self.shift_start(period_paid_for), change)
    }
}

fn sub_amount(from: Coin, amount: Uint128) -> Coin {
    Coin {
        amount: from.amount - amount,
        denom: from.denom,
    }
}

fn add_coin(to: Coin, other: Coin) -> Coin {
    debug_assert!(to.denom == other.denom);
    Coin {
        amount: to.amount + other.amount,
        denom: to.denom,
    }
}

fn fraction<R, S, T>(what: R, shares: S, total: T) -> R
where
    R: Into<Uint128> + TryFrom<u128>,
    <R as TryFrom<u128>>::Error: Debug,
    S: Into<u128> + PartialEq<T>,
    T: Into<u128>,
{
    if shares == total {
        what
    } else {
        let w: Uint128 = what.into();
        w.multiply_ratio(shares.into(), total.into())
            .u128()
            .try_into()
            .expect("Overflow computing a fraction")
    }
}
