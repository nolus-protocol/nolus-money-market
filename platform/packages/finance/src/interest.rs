use std::{cmp, fmt::Debug, ops::Sub};

use crate::{duration::Duration, fraction::Fraction, fractionable::Fractionable, zero::Zero};

/// Computes how much interest is accrued
pub fn interest<U, F, P>(rate: F, principal: P, period: Duration) -> P
where
    F: Fraction<U>,
    P: Fractionable<U>,
{
    todo!("Reimplement");
    let interest_per_year = rate.of(principal);
    period.annualized_slice_of(interest_per_year)
}

/// Computes how much time this payment covers, return.0, and the change, return.1
///
/// The actual payment is equal to the payment minus the returned change.
pub fn pay<U, F, P>(rate: F, principal: P, payment: P, period: Duration) -> (Duration, P)
where
    F: Fraction<U>,
    P: Copy + Debug + Fractionable<U> + Ord + Sub<Output = P> + Zero,
    Duration: Fractionable<P>,
{
    todo!("Reimplement");

    let interest_due_per_period: P = interest(rate, principal, period);

    if interest_due_per_period == P::ZERO {
        (Duration::from_nanos(0), payment)
    } else {
        let repayment: P = cmp::min(interest_due_per_period, payment);

        let period_paid_for = period.into_slice_per_ratio(repayment, interest_due_per_period);
        let change = payment - repayment;
        (period_paid_for, change)
    }
}

#[cfg(test)]
mod tests {
    use currency::test::SubGroupTestC10;

    use crate::{
        coin::Coin, duration::Duration, fraction::Fraction, percent::Percent, ratio::Rational,
        zero::Zero,
    };

    type MyCoin = Coin<SubGroupTestC10>;
    const PERIOD_LENGTH: Duration = Duration::YEAR;

    #[test]
    fn pay_zero_principal() {
        let p = Percent::from_percent(10);
        let principal = MyCoin::ZERO;
        let payment = MyCoin::new(300);
        pay_impl(
            p,
            principal,
            payment,
            PERIOD_LENGTH,
            Duration::default(),
            payment,
        );
    }

    #[test]
    fn pay_zero_payment() {
        let p = Percent::from_percent(10);
        let principal = MyCoin::new(1000);
        let payment = MyCoin::ZERO;
        pay_impl(
            p,
            principal,
            payment,
            PERIOD_LENGTH,
            Duration::default(),
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
            PERIOD_LENGTH,
            PERIOD_LENGTH,
            exp_change,
        );

        pay_impl(
            p,
            principal,
            payment,
            Duration::default(),
            Duration::default(),
            payment,
        );
    }

    #[test]
    fn pay_all_due() {
        let p = Percent::from_percent(10);
        let principal = MyCoin::new(1000);
        let payment = MyCoin::new(300);
        let exp_change = payment - p.of(principal);
        pay_impl(
            p,
            principal,
            payment,
            PERIOD_LENGTH,
            PERIOD_LENGTH,
            exp_change,
        );
    }

    #[test]
    fn pay_zero_due_does_not_touch_the_period() {
        let p = Percent::from_percent(10);
        let principal = MyCoin::new(9); // 10% of 9 = 0
        let payment = MyCoin::new(100);
        let exp_change = payment - p.of(principal);
        pay_impl(
            p,
            principal,
            payment,
            PERIOD_LENGTH,
            Duration::default(),
            exp_change,
        );
    }

    #[test]
    fn interest() {
        let whole = MyCoin::new(1001);
        let part = MyCoin::new(125);
        let r = Rational::new(part, whole);

        let res = super::interest::<MyCoin, _, _>(r, whole, PERIOD_LENGTH);
        assert_eq!(part, res);
    }

    #[test]
    fn interest_zero() {
        let principal = MyCoin::new(1001);
        let r = Rational::new(MyCoin::ZERO, principal);

        let res = super::interest::<MyCoin, _, _>(r, principal, PERIOD_LENGTH);
        assert_eq!(MyCoin::ZERO, res);
    }

    fn pay_impl(
        rate: Percent,
        principal: MyCoin,
        payment: MyCoin,
        pay_for: Duration,
        exp_paid_for: Duration,
        exp_change: MyCoin,
    ) {
        let (paid_for, change) = super::pay(rate, principal, payment, pay_for);
        assert_eq!(exp_paid_for, paid_for);
        assert_eq!(exp_change, change);
    }
}
