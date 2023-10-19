use std::{cmp, fmt::Debug, marker::PhantomData, ops::Sub};

use serde::{Deserialize, Serialize};

use sdk::cosmwasm_std::Timestamp;

use crate::{
    duration::Duration,
    fraction::Fraction,
    fractionable::{Fractionable, TimeSliceable},
    period::Period,
    zero::Zero,
};

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub struct InterestPeriod<U, F> {
    period: Period,
    #[serde(skip)]
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
            period: Period::default(),
            interest_units: PhantomData,
            interest,
        }
    }

    pub fn and_period(self, period: Period) -> Self {
        Self {
            period,
            interest_units: self.interest_units,
            interest: self.interest,
        }
    }

    pub fn zero_length(&self) -> bool {
        self.period.length() == Duration::default()
    }

    pub fn period(&self) -> Period {
        self.period
    }

    pub fn start(&self) -> Timestamp {
        self.period.start()
    }

    pub fn till(&self) -> Timestamp {
        self.period.till()
    }

    pub fn interest_rate(&self) -> F {
        self.interest
    }

    pub fn interest<P>(&self, principal: P) -> P
    where
        P: Fractionable<U> + TimeSliceable,
    {
        self.interest_by(principal, self.till())
    }

    ///
    /// The return.1 is the change after the payment. The actual payment is
    /// equal to the payment minus the returned change.
    pub fn pay<P>(self, principal: P, payment: P, by: Timestamp) -> (Self, P)
    where
        P: Zero + Debug + Copy + Ord + Sub<Output = P> + Fractionable<U> + TimeSliceable,
        Duration: Fractionable<P>,
    {
        let by_within_period = self.move_within_period(by);
        let interest_due_per_period = self.interest_by(principal, by_within_period);

        if interest_due_per_period == P::ZERO {
            (self, payment)
        } else {
            let repayment = cmp::min(interest_due_per_period, payment);

            let period = Duration::between(self.start(), by_within_period);
            let period_paid_for = period.into_slice_per_ratio(repayment, interest_due_per_period);

            let change = payment - repayment;
            (self.shift_start(period_paid_for), change)
        }
    }

    #[track_caller]
    fn shift_start(self, delta: Duration) -> Self {
        let res = Self::with_interest(self.interest).and_period(self.period.shift_start(delta));
        debug_assert_eq!(self.till(), res.till());
        res
    }

    fn move_within_period(&self, t: Timestamp) -> Timestamp {
        t.clamp(self.start(), self.till())
    }

    fn interest_by<P>(&self, principal: P, by: Timestamp) -> P
    where
        P: Fractionable<U> + TimeSliceable,
    {
        debug_assert!(self.start() <= by);
        debug_assert!(by <= self.till());
        let period = Duration::between(self.start(), by);

        let interest_due_per_year = self.interest.of(principal);
        period.annualized_slice_of(interest_due_per_year)
    }
}

#[cfg(test)]
mod tests {
    use currency::test::SubGroupTestC1;
    use sdk::cosmwasm_std::Timestamp;

    use crate::{
        coin::Coin, duration::Duration, fraction::Fraction, percent::Percent, period::Period,
        ratio::Rational, zero::Zero,
    };

    use super::InterestPeriod;

    type MyCoin = Coin<SubGroupTestC1>;
    const PERIOD_START: Timestamp = Timestamp::from_nanos(0);
    const PERIOD_LENGTH: Duration = Duration::YEAR;

    #[test]
    fn pay_zero_principal() {
        let p = Percent::from_percent(10);
        let principal = MyCoin::ZERO;
        let payment = MyCoin::new(300);
        let by = PERIOD_START + PERIOD_LENGTH;
        pay_impl(
            p,
            principal,
            payment,
            by,
            PERIOD_START,
            PERIOD_LENGTH,
            payment,
        );
    }

    #[test]
    fn pay_zero_payment() {
        let p = Percent::from_percent(10);
        let principal = MyCoin::new(1000);
        let payment = MyCoin::ZERO;
        let by = PERIOD_START + PERIOD_LENGTH;
        pay_impl(
            p,
            principal,
            payment,
            by,
            PERIOD_START,
            PERIOD_LENGTH,
            payment,
        );
    }

    #[test]
    fn pay_outside_period() {
        let p = Percent::from_percent(10);
        let principal = MyCoin::new(1000);
        let payment = MyCoin::new(345);
        let exp_change = payment - p.of(principal);
        pay_impl(
            p,
            principal,
            payment,
            PERIOD_START + PERIOD_LENGTH + PERIOD_LENGTH,
            PERIOD_START + PERIOD_LENGTH,
            Duration::from_nanos(0),
            exp_change,
        );

        pay_impl(
            p,
            principal,
            payment,
            PERIOD_START,
            PERIOD_START,
            PERIOD_LENGTH,
            payment,
        );
    }

    #[test]
    fn pay_all_due() {
        let p = Percent::from_percent(10);
        let principal = MyCoin::new(1000);
        let payment = MyCoin::new(300);
        let by = PERIOD_START + PERIOD_LENGTH;
        let exp_change = payment - p.of(principal);
        pay_impl(
            p,
            principal,
            payment,
            by,
            by,
            Duration::from_nanos(0),
            exp_change,
        );
    }

    #[test]
    fn interest() {
        let whole = MyCoin::new(1001);
        let part = MyCoin::new(125);
        let r = Rational::new(part, whole);

        let res = ip::<MyCoin, _>(r).interest(whole);
        assert_eq!(part, res);
    }

    #[test]
    fn interest_zero() {
        let principal = MyCoin::new(1001);
        let r = Rational::new(MyCoin::ZERO, principal);

        let res = ip::<MyCoin, _>(r).interest(principal);
        assert_eq!(MyCoin::ZERO, res);
    }

    fn pay_impl(
        p: Percent,
        principal: MyCoin,
        payment: MyCoin,
        by: Timestamp,
        exp_start: Timestamp,
        exp_length: Duration,
        exp_change: MyCoin,
    ) {
        let ip = ip(p);
        assert_eq!(p, ip.interest_rate());

        let (ip_res, change) = ip.pay(principal, payment, by);
        let ip_exp =
            InterestPeriod::with_interest(p).and_period(Period::from_length(exp_start, exp_length));
        assert_eq!(ip_exp, ip_res);
        assert_eq!(exp_change, change);
        assert_eq!(p, ip_res.interest_rate());
    }

    fn ip<U, F>(fraction: F) -> InterestPeriod<U, F>
    where
        U: PartialEq,
        F: Copy + Fraction<U>,
    {
        InterestPeriod::with_interest(fraction)
            .and_period(Period::from_length(PERIOD_START, PERIOD_LENGTH))
    }
}
