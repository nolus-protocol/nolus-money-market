use serde::{Deserialize, Serialize};

use finance::instant::Instant;
use finance::{coin::Coin, duration::Duration, interest, percent::Percent100};
#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
#[cfg_attr(any(test, feature = "testing"), derive(Eq, PartialEq))]
#[serde(rename_all = "snake_case", bound(serialize = "", deserialize = ""))]
pub struct Loan<Lpn> {
    pub principal_due: Coin<Lpn>,
    pub annual_interest_rate: Percent100,
    pub interest_paid: Instant,
}

#[derive(Debug, Default, Eq, PartialEq)]
pub struct RepayShares<Lpn>
where
    Lpn: 'static,
{
    pub interest: Coin<Lpn>,
    pub principal: Coin<Lpn>,
    pub excess: Coin<Lpn>,
}

impl<Lpn> Loan<Lpn> {
    pub fn interest_due(&self, by: &Instant) -> Option<Coin<Lpn>> {
        interest::interest(
            self.annual_interest_rate,
            self.principal_due,
            self.due_period(by),
        )
    }

    pub fn repay(&mut self, by: &Instant, repayment: Coin<Lpn>) -> Option<RepayShares<Lpn>> {
        interest::pay(
            self.annual_interest_rate,
            self.principal_due,
            repayment,
            self.due_period(by),
        )
        .map(|(paid_for, interest_change)| {
            let interest_paid = repayment - interest_change;
            let principal_paid = interest_change.min(self.principal_due);
            let excess = interest_change - principal_paid;

            self.principal_due -= principal_paid;
            self.interest_paid += paid_for;

            RepayShares {
                interest: interest_paid,
                principal: principal_paid,
                excess,
            }
        })
    }

    fn due_period(&self, by: &Instant) -> Duration {
        Duration::between(&self.interest_paid, by.max(&self.interest_paid))
    }
}

#[cfg(test)]
mod test {
    use crate::loan::{Loan, RepayShares};
    use currencies::Lpn;
    use finance::instant::Instant;
    use finance::{
        coin::{Amount, Coin},
        duration::Duration,
        fraction::Fraction,
        percent::Percent100,
        zero::Zero,
    };

    #[test]
    fn interest() {
        let l = Loan {
            principal_due: lpn_coin(100),
            annual_interest_rate: Percent100::from_percent(50),
            interest_paid: Instant::from_nanos(200),
        };

        assert_eq!(
            Some(lpn_coin(50)),
            l.interest_due(&(l.interest_paid + Duration::YEAR))
        );

        assert_eq!(Some(Coin::ZERO), l.interest_due(&l.interest_paid));
        assert_eq!(
            Some(Coin::ZERO),
            l.interest_due(&l.interest_paid.minus_nanos(1))
        );
    }

    #[test]
    fn repay_no_interest() {
        let principal_at_start = lpn_coin(500);
        let interest = Percent100::from_percent(50);
        let start_at = Instant::from_nanos(200);
        let interest_paid = start_at;
        let mut l = Loan {
            principal_due: principal_at_start,
            annual_interest_rate: interest,
            interest_paid,
        };

        let payment1 = lpn_coin(10);
        assert_eq!(
            Some(RepayShares {
                interest: Coin::ZERO,
                principal: payment1,
                excess: Coin::ZERO
            }),
            l.repay(&interest_paid, payment1)
        );
        assert_eq!(
            Loan {
                principal_due: principal_at_start - payment1,
                annual_interest_rate: interest,
                interest_paid: l.interest_paid
            },
            l
        );
    }

    #[test]
    fn repay_interest_only() {
        let principal_start = lpn_coin(500);
        let interest = Percent100::from_percent(50);
        let mut l = Loan {
            principal_due: principal_start,
            annual_interest_rate: interest,
            interest_paid: Instant::from_nanos(200),
        };

        let interest_a_year = interest.of(principal_start);
        let at_first_year_end = l.interest_paid + Duration::YEAR;
        assert_eq!(
            Some(RepayShares {
                interest: interest_a_year,
                principal: Coin::ZERO,
                excess: Coin::ZERO
            }),
            l.repay(&at_first_year_end, interest_a_year)
        );
        assert_eq!(
            Loan {
                principal_due: principal_start,
                annual_interest_rate: interest,
                interest_paid: at_first_year_end
            },
            l
        );
    }

    #[test]
    fn repay_all() {
        let principal_start = lpn_coin(50000000000);
        let interest = Percent100::from_percent(50);
        let mut l = Loan {
            principal_due: principal_start,
            annual_interest_rate: interest,
            interest_paid: Instant::from_nanos(200),
        };

        let interest_a_year = interest.of(principal_start);
        let at_first_hour_end = l.interest_paid + Duration::HOUR;
        let exp_interest = interest_a_year.checked_div(365 * 24).unwrap();
        let excess = lpn_coin(12441);
        assert_eq!(
            Some(RepayShares {
                interest: exp_interest,
                principal: principal_start,
                excess,
            }),
            l.repay(&at_first_hour_end, exp_interest + principal_start + excess)
        );
        assert_eq!(
            Loan {
                principal_due: Coin::ZERO,
                annual_interest_rate: interest,
                interest_paid: at_first_hour_end
            },
            l
        );
    }

    const fn lpn_coin(amount: Amount) -> Coin<Lpn> {
        Coin::new(amount)
    }
}
