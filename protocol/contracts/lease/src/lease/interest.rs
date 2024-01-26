use currency::Currency;
use finance::{coin::Coin, duration::Duration, fraction::Fraction};

use crate::{loan::State, position::InterestDue};

impl<Lpn> InterestDue<Lpn> for State<Lpn>
where
    Lpn: Currency,
{
    fn time_to_get_to(&self, min_due_interest: Coin<Lpn>) -> Duration {
        let total_due_interest = self.due_interest
            + self.due_margin_interest
            + self.overdue.interest()
            + self.overdue.margin();
        let time_to_accrue_min_due_interest = if total_due_interest >= min_due_interest {
            Duration::default()
        } else {
            let overdue_left = min_due_interest - total_due_interest;

            // TODO define the following as a fn in InterestPayment and replace all occurences
            let total_interest_a_year =
                (self.annual_interest + self.annual_interest_margin).of(self.principal_due);
            Duration::YEAR.into_slice_per_ratio(overdue_left, total_interest_a_year)
        };
        self.overdue.start_in().max(time_to_accrue_min_due_interest)
    }
}

#[cfg(test)]
mod test {
    use finance::{duration::Duration, interest, percent::Percent};

    use crate::{
        lease::tests::TestLpn,
        loan::{Overdue, State},
        position::InterestDue,
    };

    #[test]
    fn already_above_the_limit_before_due_end() {
        let due_interest = 10.into();
        let due_margin_interest = 5.into();
        let till_due_end = Duration::from_days(3);
        let s = State {
            annual_interest: Percent::from_percent(20),
            annual_interest_margin: Percent::from_percent(5),
            principal_due: 100_000.into(),
            due_interest,
            due_margin_interest,
            overdue: Overdue::<TestLpn>::StartIn(till_due_end),
        };
        assert_eq!(
            till_due_end,
            s.time_to_get_to(due_interest + due_margin_interest - 1.into())
        );
    }

    #[test]
    fn get_to_limit_before_due_end() {
        let annual_interest = Percent::from_percent(20);
        let annual_interest_margin = Percent::from_percent(5);
        let principal_due = 100_000.into();
        let due_interest = 10.into();
        let due_margin_interest = 5.into();
        let till_due_end = Duration::from_days(3);
        let delta_to_due_end = interest::interest(
            annual_interest + annual_interest_margin,
            principal_due,
            till_due_end,
        );
        let s = State {
            annual_interest,
            annual_interest_margin,
            principal_due,
            due_interest,
            due_margin_interest,
            overdue: Overdue::<TestLpn>::StartIn(till_due_end),
        };
        assert_eq!(
            till_due_end,
            s.time_to_get_to(due_interest + due_margin_interest + delta_to_due_end - 1.into())
        );
    }

    #[test]
    fn below_the_limit_past_due_end() {
        let annual_interest = Percent::from_percent(20);
        let annual_interest_margin = Percent::from_percent(5);
        let principal_due = 100_000.into();
        let due_interest = 15.into();
        let due_margin_interest = 5.into();
        let overdue_interest = 7.into();
        let overdue_margin_interest = 2.into();
        let total_interest =
            due_interest + due_margin_interest + overdue_interest + overdue_margin_interest;

        let delta_to_overdue = 40.into();
        let till_overdue = Duration::YEAR.into_slice_per_ratio(
            delta_to_overdue,
            interest::interest(
                annual_interest + annual_interest_margin,
                principal_due,
                Duration::YEAR,
            ),
        );

        let s = State {
            annual_interest,
            annual_interest_margin,
            principal_due,
            due_interest,
            due_margin_interest,
            overdue: Overdue::<TestLpn>::Accrued {
                interest: overdue_interest,
                margin: overdue_margin_interest,
            },
        };
        assert_eq!(
            till_overdue,
            s.time_to_get_to(total_interest + delta_to_overdue)
        );
    }

    #[test]
    fn above_the_limit_past_due_end() {
        let annual_interest = Percent::from_percent(20);
        let annual_interest_margin = Percent::from_percent(5);
        let principal_due = 100_000.into();
        let due_interest = 15.into();
        let due_margin_interest = 5.into();
        let overdue_interest = 7.into();
        let overdue_margin_interest = 2.into();
        let total_interest =
            due_interest + due_margin_interest + overdue_interest + overdue_margin_interest;

        let s = State {
            annual_interest,
            annual_interest_margin,
            principal_due,
            due_interest,
            due_margin_interest,
            overdue: Overdue::<TestLpn>::Accrued {
                interest: overdue_interest,
                margin: overdue_margin_interest,
            },
        };
        assert_eq!(
            Duration::default(),
            s.time_to_get_to(total_interest - 1.into())
        );
    }
}
