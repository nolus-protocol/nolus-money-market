use cosmwasm_std::{Coin, Timestamp, Uint128};
use serde::{Deserialize, Serialize};
use std::{cmp, fmt::Debug};

use crate::{coin, duration::Duration, percent::Percent};

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub struct InterestPeriod {
    start: Timestamp,
    length: Duration,
    interest: Percent,
}

impl InterestPeriod {
    pub fn with_interest(interest: Percent) -> Self {
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

    pub fn start(&self) -> Timestamp {
        self.start
    }

    pub fn till(&self) -> Timestamp {
        self.start + self.length
    }

    pub fn interest(&self, principal: Coin) -> Coin {
        self.interest_by(principal, self.till())
    }

    pub fn pay(self, principal: Coin, payment: Coin, by: Timestamp) -> (Self, Coin) {
        let by_within_period = self.move_within_period(by);
        let interest_due_per_period = self.interest_by(principal, by_within_period);

        let period = Duration::between(self.start, by_within_period);
        let repayment = cmp::min(interest_due_per_period.amount, payment.amount);
        let period_paid_for = fraction(period, repayment, interest_due_per_period.amount);

        let change = coin::sub_amount(payment, repayment);
        (self.shift_start(period_paid_for), change)
    }

    fn move_within_period(&self, t: Timestamp) -> Timestamp {
        cmp::min(cmp::max(self.start, t), self.till())
    }

    fn interest_by(&self, principal: Coin, by: Timestamp) -> Coin {
        debug_assert!(self.start <= by);
        debug_assert!(by <= self.till());
        let period = Duration::between(self.start, by);

        let interest_due_per_year = self.interest.of(principal);
        let interest_due_per_period =
            fraction(interest_due_per_year.amount, period, Duration::YEAR);
        Coin {
            amount: interest_due_per_period,
            denom: interest_due_per_year.denom,
        }
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
