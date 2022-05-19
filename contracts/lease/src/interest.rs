use std::{cmp, fmt::Debug};
use cosmwasm_std::{Timestamp, Coin, Uint128};
use finance::percent::Percent;
use serde::{Serialize, Deserialize};

use crate::{duration::Duration, coin};

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub struct InterestPeriod {
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

        let change = coin::sub_amount(payment, repayment);
        (self.shift_start(period_paid_for), change)
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