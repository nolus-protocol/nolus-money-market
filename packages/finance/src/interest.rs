use cosmwasm_std::Timestamp;
use serde::{Deserialize, Serialize};
use std::{cmp, fmt::Debug, marker::PhantomData, ops::Sub};

use crate::{
    duration::Duration,
    fraction::Fraction,
    fractionable::{Fractionable, TimeSliceable},
};
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub struct InterestPeriod<U, F> {
    start: Timestamp,
    length: Duration,
    interest_units: PhantomData<U>,
    interest: F,
}

impl<U, F> InterestPeriod<U, F>
where
    F: Fraction<U> + Copy,
    U: PartialEq,
{
    pub fn with_interest(interest: F) -> Self {
        Self {
            start: Timestamp::default(),
            length: Duration::default(),
            interest_units: PhantomData,
            interest,
        }
    }

    pub fn from(self, start: Timestamp) -> Self {
        Self {
            start,
            length: self.length,
            interest_units: self.interest_units,
            interest: self.interest,
        }
    }

    pub fn spanning(self, length: Duration) -> Self {
        Self {
            start: self.start,
            length,
            interest_units: self.interest_units,
            interest: self.interest,
        }
    }

    pub fn shift_start(self, delta: Duration) -> Self {
        assert!(delta <= self.length);
        let res = Self {
            start: self.start + delta,
            length: self.length - delta,
            interest_units: self.interest_units,
            interest: self.interest,
        };
        debug_assert_eq!(self.till(), res.till());
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
        P: Fractionable<U> + TimeSliceable,
    {
        self.interest_by(principal, self.till())
    }

    pub fn annual_interest_rate(&self) -> F {
        self.interest
    }

    ///
    /// The return.1 is the change after the payment. The actual payment is
    /// equal to the payment minus the returned change.
    pub fn pay<P>(self, principal: P, payment: P, by: Timestamp) -> (Self, P)
    where
        P: Default + Copy + Ord + Sub<Output = P> + Fractionable<U> + TimeSliceable,
        Duration: Fractionable<P>,
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
        P: Fractionable<U> + TimeSliceable,
    {
        debug_assert!(self.start <= by);
        debug_assert!(by <= self.till());
        let period = Duration::between(self.start, by);

        let interest_due_per_year = self.interest.of(principal);
        period.annualized_slice_of(interest_due_per_year)
    }
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::Timestamp;

    use crate::{
        coin::Coin,
        currency::{Nls, Usdc},
        duration::Duration,
        fraction::Fraction,
        percent::Percent,
        ratio::Rational,
    };

    use super::InterestPeriod;

    #[test]
    fn pay() {
        let p = Percent::from_percent(10);
        let principal = Coin::<Usdc>::new(1000);
        let payment = Coin::<Usdc>::new(200);
        let ip = InterestPeriod::with_interest(p)
            .from(Timestamp::from_nanos(0))
            .spanning(Duration::YEAR);
        let (ip_res, change) = ip.pay(principal, payment, ip.till());
        let ip_exp = InterestPeriod::with_interest(p)
            .from(ip.till())
            .spanning(Duration::from_secs(0));
        assert_eq!(ip_exp, ip_res);
        assert_eq!(payment - p.of(principal), change);
    }

    #[test]
    fn interest() {
        type MyCoin = Coin<Nls>;
        let whole = MyCoin::new(1001);
        let part = MyCoin::new(125);
        let r = Rational::new(part, whole);

        let res = InterestPeriod::<MyCoin, Rational<MyCoin>>::with_interest(r)
            .from(Timestamp::from_nanos(0))
            .spanning(Duration::YEAR)
            .interest(whole);
        assert_eq!(part, res);
    }
}
