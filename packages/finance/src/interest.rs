use cosmwasm_std::Timestamp;
use serde::{Deserialize, Serialize};
use std::{cmp, fmt::Debug, ops::{Sub, Mul, Div}};

use crate::{
    duration::{Duration, Units as TimeUnits},
    percent::Percent,
    percentable::{Percentable, TimeSliceable, Integer},
};
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

    pub fn interest<P>(&self, principal: P) -> P
    where
        P: Percentable + TimeSliceable,
    {
        self.interest_by(principal, self.till())
    }

    pub fn pay<P,D>(self, principal: P, payment: P, by: Timestamp) -> (Self, P)
    where
        P: Percentable + TimeSliceable + Ord + Default + Sub<Output = P> + Copy,
        TimeUnits: Integer<DoubleInteger = D> + TryFrom<D>,
        D: From<TimeUnits> + From<P> + Mul<D, Output = D> + Div<D, Output = D>,
        <TimeUnits as TryFrom<D>>::Error: Debug,

    {
        let by_within_period = self.move_within_period(by);
        let interest_due_per_period = self.interest_by(principal, by_within_period);

        let period = Duration::between(self.start, by_within_period);
        let repayment = cmp::min(interest_due_per_period, payment);
        let period_paid_for = period.into_slice_per_ratio(repayment, interest_due_per_period);

        let change = payment - repayment;
        (self.shift_start(period_paid_for), change)
    }

    fn move_within_period(&self, t: Timestamp) -> Timestamp {
        t.clamp(self.start, self.till())
    }

    fn interest_by<P>(&self, principal: P, by: Timestamp) -> P
    where
        P: Percentable + TimeSliceable,
    {
        debug_assert!(self.start <= by);
        debug_assert!(by <= self.till());
        let period = Duration::between(self.start, by);

        let interest_due_per_year = self.interest.of(principal);
        period.annualized_slice_of(interest_due_per_year)
    }
}