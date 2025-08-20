use finance::{duration::Duration, interest};

use crate::{
    finance::LpnCoin,
    loan::State,
    position::{DueTrait, OverdueCollection},
};

impl DueTrait for State {
    fn total_due(&self) -> LpnCoin {
        self.principal_due + self.total_due_interest()
    }

    fn overdue_collection(&self, min_amount: LpnCoin) -> OverdueCollection {
        let total_due_interest = self.total_due_interest();
        let time_to_accrue_min_amount = if total_due_interest >= min_amount {
            Duration::default()
        } else {
            let overdue_left = min_amount - total_due_interest;

            let total_interest_a_year = interest::interest(
                self.annual_interest
                    .checked_add(self.annual_interest_margin)
                    .expect("TODO: propagate up the stack potential overflow"),
                self.principal_due,
                Duration::YEAR,
            );
            if total_interest_a_year.is_zero() {
                Duration::MAX
            } else {
                Duration::YEAR.into_slice_per_ratio(overdue_left, total_interest_a_year)
            }
        };
        let time_to_collect = self.overdue.start_in().max(time_to_accrue_min_amount);
        if time_to_collect == Duration::default() {
            OverdueCollection::Overdue(total_due_interest)
        } else {
            OverdueCollection::StartIn(time_to_collect)
        }
    }
}

impl State {
    fn total_due_interest(&self) -> LpnCoin {
        self.due_interest
            + self.due_margin_interest
            + self.overdue.interest()
            + self.overdue.margin()
    }
}

#[cfg(all(feature = "internal.test.contract", test))]
mod test {

    use finance::{coin::Coin, duration::Duration, interest, percent::Percent100, zero::Zero};

    use crate::{
        lease::tests,
        loan::{Overdue, State},
        position::DueTrait,
    };

    #[test]
    fn already_above_the_limit_before_due_end() {
        let principal_due = tests::lpn_coin(100_000);
        let due_interest = tests::lpn_coin(10);
        let due_margin_interest = tests::lpn_coin(5);
        let till_due_end = Duration::from_days(3);
        let s = State {
            annual_interest: Percent100::from_percent(20),
            annual_interest_margin: Percent100::from_percent(5),
            principal_due,
            due_interest,
            due_margin_interest,
            overdue: Overdue::StartIn(till_due_end),
        };
        let overdue_collection =
            s.overdue_collection(due_interest + due_margin_interest - tests::lpn_coin(1));
        assert_eq!(till_due_end, overdue_collection.start_in());
        assert_eq!(Coin::ZERO, overdue_collection.amount());
        assert_eq!(
            principal_due + due_interest + due_margin_interest,
            s.total_due()
        );
    }

    #[test]
    fn get_to_limit_before_due_end() {
        let annual_interest = Percent::from_percent(20);
        let annual_interest_margin = Percent::from_percent(5);
        let principal_due = tests::lpn_coin(100_000);
        let due_interest = tests::lpn_coin(10);
        let due_margin_interest = tests::lpn_coin(5);
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
            overdue: Overdue::StartIn(till_due_end),
        };
        let overdue_collection = s.overdue_collection(
            due_interest + due_margin_interest + delta_to_due_end - tests::lpn_coin(1),
        );
        assert_eq!(till_due_end, overdue_collection.start_in());
        assert_eq!(Coin::ZERO, overdue_collection.amount());
        assert_eq!(
            principal_due + due_interest + due_margin_interest,
            s.total_due()
        );
    }

    #[test]
    fn below_the_limit_past_due_end() {
        let annual_interest = Percent::from_percent(20);
        let annual_interest_margin = Percent::from_percent(5);
        let principal_due = tests::lpn_coin(100_000);
        let due_interest = tests::lpn_coin(15);
        let due_margin_interest = tests::lpn_coin(5);
        let overdue_interest = tests::lpn_coin(7);
        let overdue_margin_interest = tests::lpn_coin(2);
        let total_interest =
            due_interest + due_margin_interest + overdue_interest + overdue_margin_interest;

        let delta_to_overdue = tests::lpn_coin(40);
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
            overdue: Overdue::Accrued {
                interest: overdue_interest,
                margin: overdue_margin_interest,
            },
        };
        let overdue_collection = s.overdue_collection(total_interest + delta_to_overdue);
        assert_eq!(till_overdue, overdue_collection.start_in());
        assert_eq!(Coin::ZERO, overdue_collection.amount());
        assert_eq!(principal_due + total_interest, s.total_due());
    }

    #[test]
    fn above_the_limit_past_due_end() {
        let annual_interest = Percent::from_percent(20);
        let annual_interest_margin = Percent::from_percent(5);
        let principal_due = tests::lpn_coin(100_000);
        let due_interest = tests::lpn_coin(15);
        let due_margin_interest = tests::lpn_coin(5);
        let overdue_interest = tests::lpn_coin(7);
        let overdue_margin_interest = tests::lpn_coin(2);
        let total_interest =
            due_interest + due_margin_interest + overdue_interest + overdue_margin_interest;

        let s = State {
            annual_interest,
            annual_interest_margin,
            principal_due,
            due_interest,
            due_margin_interest,
            overdue: Overdue::Accrued {
                interest: overdue_interest,
                margin: overdue_margin_interest,
            },
        };
        let overdue_collection = s.overdue_collection(total_interest - tests::lpn_coin(1));
        assert_eq!(Duration::default(), overdue_collection.start_in());
        assert_eq!(total_interest, overdue_collection.amount());
        assert_eq!(principal_due + total_interest, s.total_due());
    }

    #[test]
    fn fully_paid_lease_no_collectable_overdue() {
        let principal_due = Coin::ZERO;
        let due_interest = Coin::ZERO;
        let due_margin_interest = Coin::ZERO;
        let overdue_interest = Coin::ZERO;
        let overdue_margin_interest = Coin::ZERO;
        let total_interest =
            due_interest + due_margin_interest + overdue_interest + overdue_margin_interest;

        let overdue_start_in = Duration::from_days(6);
        let s = State {
            annual_interest: Percent100::from_percent(20),
            annual_interest_margin: Percent100::from_percent(5),
            principal_due,
            due_interest,
            due_margin_interest,
            overdue: Overdue::StartIn(overdue_start_in),
        };
        let overdue_collection = s.overdue_collection(tests::lpn_coin(100));
        assert_eq!(Duration::MAX, overdue_collection.start_in());
        assert_eq!(Coin::ZERO, overdue_collection.amount());
        assert_eq!(principal_due + total_interest, s.total_due());
    }

    #[test]
    fn fully_paid_lease_with_collectable_overdue() {
        let principal_due = Coin::ZERO;
        let due_interest = Coin::ZERO;
        let due_margin_interest = Coin::ZERO;
        let overdue_interest = Coin::ZERO;
        let overdue_margin_interest = Coin::ZERO;
        let total_interest =
            due_interest + due_margin_interest + overdue_interest + overdue_margin_interest;

        let s = State {
            annual_interest: Percent100::from_percent(20),
            annual_interest_margin: Percent100::from_percent(5),
            principal_due,
            due_interest,
            due_margin_interest,
            overdue: Overdue::Accrued {
                interest: overdue_interest,
                margin: overdue_margin_interest,
            },
        };
        let overdue_collection = s.overdue_collection(tests::lpn_coin(100));
        assert_eq!(Duration::MAX, overdue_collection.start_in());
        assert_eq!(Coin::ZERO, overdue_collection.amount());
        assert_eq!(principal_due + total_interest, s.total_due());
    }
}
