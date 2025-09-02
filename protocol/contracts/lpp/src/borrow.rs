use serde::{Deserialize, Serialize};

use finance::{
    coin::Coin,
    percent::{Percent, Percent100, Units as PercentUnits},
    ratio::SimpleFraction,
    rational::Rational,
};

#[derive(Debug, Copy, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "UncheckedInterestRate")]
pub struct InterestRate {
    base_interest_rate: Percent100,
    utilization_optimal: Percent100,
    addon_optimal_interest_rate: Percent100,
}

impl InterestRate {
    #[cfg(any(test, feature = "testing"))]
    pub fn new(
        base_interest_rate: Percent100,
        utilization_optimal: Percent100,
        addon_optimal_interest_rate: Percent100,
    ) -> Option<Self> {
        Self::private_new(
            base_interest_rate,
            utilization_optimal,
            addon_optimal_interest_rate,
        )
    }

    fn private_new(
        base_interest_rate: Percent100,
        utilization_optimal: Percent100,
        addon_optimal_interest_rate: Percent100,
    ) -> Option<Self> {
        let value = Self {
            base_interest_rate,
            utilization_optimal,
            addon_optimal_interest_rate,
        };

        value.validate().then_some(value)
    }

    pub const fn base_interest_rate(&self) -> Percent100 {
        self.base_interest_rate
    }

    pub const fn utilization_optimal(&self) -> Percent100 {
        self.utilization_optimal
    }

    pub const fn addon_optimal_interest_rate(&self) -> Percent100 {
        self.addon_optimal_interest_rate
    }

    pub fn calculate<Lpn>(&self, total_liability: Coin<Lpn>, balance: Coin<Lpn>) -> Percent100 {
        let utilization_factor_max = Percent::from_fraction(
            self.utilization_optimal.units(),
            Percent100::HUNDRED
                .checked_sub(self.utilization_optimal)
                .expect("The optimal utilization configuration parameter should be at most 100%")
                .units(),
        )
        .expect("The utilization_max must be a valid Percent: utilization_opt < 100% ensures the ratio is valid Percent100, which always fits within Percent's wider range");

        let utilization_factor = if balance.is_zero() {
            utilization_factor_max
        } else {
            Percent::from_fraction(total_liability, balance)
                .expect("The utilization must be a valid Percent")
                .min(utilization_factor_max)
        };

        Rational::<PercentUnits>::of(
            &SimpleFraction::new(self.addon_optimal_interest_rate, self.utilization_optimal),
            utilization_factor,
        )
        .and_then(|utilization_config| {
            utilization_config
                .checked_add(self.base_interest_rate.into())
                .map(|res| Percent100::try_from(res).expect("The borrow rate must not exceed 100%"))
        })
        .expect("The utilization_config must be a valid Percent")
    }

    fn validate(&self) -> bool {
        self.utilization_optimal > Percent100::ZERO
            && self.utilization_optimal < Percent100::HUNDRED
    }
}

impl TryFrom<UncheckedInterestRate> for InterestRate {
    type Error = &'static str;

    fn try_from(value: UncheckedInterestRate) -> Result<Self, Self::Error> {
        Self::private_new(
            value.base_interest_rate,
            value.utilization_optimal,
            value.addon_optimal_interest_rate,
        )
        .ok_or("Rates should not be greater than a hundred percent!")
    }
}

#[derive(Serialize, Deserialize)]
struct UncheckedInterestRate {
    base_interest_rate: Percent100,
    utilization_optimal: Percent100,
    addon_optimal_interest_rate: Percent100,
}

#[cfg(test)]
mod tests {
    use finance::percent::Percent100;

    use crate::borrow::InterestRate;

    #[test]
    fn test_constructor() {
        assert!(
            InterestRate::new(
                Percent100::ZERO,
                Percent100::from_percent(1),
                Percent100::ZERO
            )
            .is_some(),
            ""
        );
        assert!(
            InterestRate::new(Percent100::ZERO, Percent100::HUNDRED, Percent100::ZERO).is_none()
        );
        assert!(
            InterestRate::new(
                Percent100::from_percent(25),
                Percent100::from_percent(50),
                Percent100::from_percent(75)
            )
            .is_some()
        );
        assert!(
            InterestRate::new(
                Percent100::HUNDRED,
                Percent100::HUNDRED,
                Percent100::HUNDRED
            )
            .is_none()
        );

        assert!(InterestRate::new(Percent100::ZERO, Percent100::ZERO, Percent100::ZERO).is_none());
        assert!(
            InterestRate::new(
                Percent100::from_percent(25),
                Percent100::ZERO,
                Percent100::from_percent(75)
            )
            .is_none()
        );
        assert!(
            InterestRate::new(Percent100::HUNDRED, Percent100::ZERO, Percent100::HUNDRED).is_none()
        );
    }

    /// Test suit specifically for verifying correctness of [`InterestRate::calculate`](InterestRate::calculate).cargo fmt
    mod calculate {
        use crate::borrow::InterestRate;
        use finance::{
            coin::{Amount, Coin},
            percent::{Percent100, Units},
        };
        use lpp_platform::NLpn;

        /// Constructs an instance of [`InterestRate`].
        ///
        /// # Arguments
        ///
        /// Arguments represent rates in per milles.
        ///
        /// returns: [`InterestRate`]
        fn rate(
            base_interest_rate: u32,
            utilization_optimal: u32,
            addon_optimal_interest_rate: u32,
        ) -> InterestRate {
            let res = InterestRate::new(
                Percent100::from_permille(base_interest_rate),
                Percent100::from_permille(utilization_optimal),
                Percent100::from_permille(addon_optimal_interest_rate),
            );

            res.expect("Rates must not exceed 100%!")
        }

        fn ratio(n: Units, d: Units) -> Percent100 {
            Percent100::from_fraction(n, d).expect(
                "TODO replace with convertion from Ratio to Percent100 when Ratio become a struct",
            )
        }

        #[derive(Copy, Clone)]
        struct InOut((Amount, Amount), (Units, Units));

        fn in_out(InOut((l, b), (n, d)): InOut) -> ((Coin<NLpn>, Coin<NLpn>), Percent100) {
            ((Coin::new(l), Coin::new(b)), ratio(n, d))
        }

        fn do_test_calculate(rate: InterestRate, in_out_set: &[InOut]) {
            for ((liability, balance), output) in in_out_set.iter().copied().map(in_out) {
                assert_eq!(
                    rate.calculate(liability, balance),
                    output,
                    "Interest rate: {rate:?}\nLiability: {liability}\nBalance: {balance}",
                );
            }
        }

        #[test]
        /// Verifies that when there is no addon optimal interest rate, result is equal to the base interest rate.
        fn test_set_1() {
            for base_rate in 0..=200 {
                let rate = rate(base_rate, 500, 0);

                do_test_calculate(
                    rate,
                    &(0..=25)
                        .flat_map(|liability| {
                            (0..=25).filter_map(move |balance| {
                                (liability != 0 || balance != 0)
                                    .then_some(InOut((liability, balance), (base_rate, 1000)))
                            })
                        })
                        .collect::<Vec<_>>(),
                );
            }
        }

        #[test]
        /// Verifies that when liability is equal to zero, result is equal to the base interest rate.
        fn test_set_2() {
            for base_rate in 0..=100 {
                let rate = rate(base_rate, 500, 1000);

                do_test_calculate(
                    rate,
                    &(1..=100)
                        .map(move |balance| InOut((0, balance), (base_rate, 1000)))
                        .collect::<Vec<_>>(),
                );
            }
        }

        #[test]
        fn test_corner_set() {
            let rate = rate(300, 857, 100);

            let set = [InOut((0, 0), (999, 1000)), InOut((730, 123), (992, 1000))];
            do_test_calculate(rate, &set);
        }

        #[test]
        /// Verifies correctness of results against manually calculated, thus verified, set.
        fn test_set_4() {
            let rate = rate(100, 655, 250);

            let set = [
                InOut((10, 1), (824, 1000)),
                InOut((10, 2), (824, 1000)),
                InOut((10, 3), (824, 1000)),
                InOut((10, 4), (824, 1000)),
                InOut((10, 5), (824, 1000)),
                InOut((10, 6), (735, 1000)),
                InOut((10, 7), (645, 1000)),
                InOut((10, 8), (577, 1000)),
                InOut((10, 9), (524, 1000)),
                InOut((10, 10), (481, 1000)),
                InOut((10, 11), (446, 1000)),
                InOut((10, 12), (417, 1000)),
                InOut((10, 13), (393, 1000)),
                InOut((10, 14), (372, 1000)),
                InOut((10, 15), (354, 1000)),
            ];

            do_test_calculate(rate, &set);
        }

        #[test]
        /// Verifies correctness of results against manually calculated, thus verified, set.
        fn test_set_5() {
            let rate = rate(120, 700, 20);

            let set = [
                InOut((0, 1), (3, 25)),
                InOut((1, 9), (123, 1000)),
                InOut((3, 7), (132, 1000)),
                InOut((5, 5), (148, 1000)),
                InOut((7, 3), (186, 1000)),
                InOut((8, 2), (186, 1000)),
                InOut((9, 1), (186, 1000)),
                InOut((0, 0), (186, 1000)),
                InOut((1, 0), (186, 1000)),
            ];

            do_test_calculate(rate, &set);
        }

        #[test]
        /// Verifies correctness of results against manually calculated, thus verified, set.
        fn test_set_6() {
            let rate = rate(100, 750, 20);

            let set = [InOut((2584283, 40054 - 18571), (18, 100))];

            do_test_calculate(rate, &set);
        }
    }
}
