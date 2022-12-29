use serde::{Deserialize, Serialize};

use finance::{
    coin::Coin,
    currency::Currency,
    fraction::Fraction,
    percent::{Percent, Units},
    ratio::Rational,
};
use sdk::schemars::{self, JsonSchema};

#[derive(Debug, Copy, Clone, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(try_from = "UncheckedInterestRate")]
pub struct InterestRate {
    base_interest_rate: Percent,
    utilization_optimal: Percent,
    addon_optimal_interest_rate: Percent,
}

impl InterestRate {
    #[cfg(any(test, feature = "testing"))]
    pub fn new(
        base_interest_rate: Percent,
        utilization_optimal: Percent,
        addon_optimal_interest_rate: Percent,
    ) -> Option<Self> {
        Self::private_new(
            base_interest_rate,
            utilization_optimal,
            addon_optimal_interest_rate,
        )
    }

    fn private_new(
        base_interest_rate: Percent,
        utilization_optimal: Percent,
        addon_optimal_interest_rate: Percent,
    ) -> Option<Self> {
        let value = Self {
            base_interest_rate,
            utilization_optimal,
            addon_optimal_interest_rate,
        };

        value.validate().then_some(value)
    }

    pub fn base_interest_rate(&self) -> Percent {
        self.base_interest_rate
    }

    pub fn utilization_optimal(&self) -> Percent {
        self.utilization_optimal
    }

    pub fn addon_optimal_interest_rate(&self) -> Percent {
        self.addon_optimal_interest_rate
    }

    pub fn calculate<Lpn>(&self, total_liability: Coin<Lpn>, balance: Coin<Lpn>) -> Percent
    where
        Lpn: Currency,
    {
        let total_value = total_liability + balance;
        let utilization = if total_value.is_zero() {
            Percent::ZERO
        } else {
            Percent::from_ratio(total_liability, total_value)
        };

        let config = Rational::new(
            self.addon_optimal_interest_rate.units(),
            self.utilization_optimal.units(),
        );

        let add = Fraction::<Units>::of(&config, utilization);

        self.base_interest_rate + add
    }

    fn validate(&self) -> bool {
        self.base_interest_rate <= Percent::HUNDRED
            && self.utilization_optimal > Percent::ZERO
            && self.utilization_optimal <= Percent::HUNDRED
            && self.addon_optimal_interest_rate <= Percent::HUNDRED
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

#[derive(Serialize, Deserialize, JsonSchema)]
struct UncheckedInterestRate {
    base_interest_rate: Percent,
    utilization_optimal: Percent,
    addon_optimal_interest_rate: Percent,
}

#[cfg(test)]
mod tests {
    use crate::borrow::InterestRate;
    use finance::percent::Percent;

    #[test]
    fn test_constructor() {
        assert!(
            InterestRate::new(Percent::ZERO, Percent::from_percent(1), Percent::ZERO).is_some(),
            ""
        );
        assert!(InterestRate::new(Percent::ZERO, Percent::HUNDRED, Percent::ZERO).is_some());
        assert!(InterestRate::new(
            Percent::from_percent(25),
            Percent::from_percent(50),
            Percent::from_percent(75)
        )
        .is_some());
        assert!(InterestRate::new(Percent::HUNDRED, Percent::HUNDRED, Percent::HUNDRED).is_some());

        assert!(InterestRate::new(Percent::ZERO, Percent::ZERO, Percent::ZERO).is_none());
        assert!(InterestRate::new(
            Percent::from_percent(25),
            Percent::ZERO,
            Percent::from_percent(75)
        )
        .is_none());
        assert!(InterestRate::new(Percent::HUNDRED, Percent::ZERO, Percent::HUNDRED).is_none());
        assert!(InterestRate::new(
            Percent::from_percent(101),
            Percent::HUNDRED,
            Percent::HUNDRED
        )
        .is_none());
        assert!(InterestRate::new(
            Percent::HUNDRED,
            Percent::from_percent(101),
            Percent::HUNDRED
        )
        .is_none());
        assert!(InterestRate::new(
            Percent::HUNDRED,
            Percent::HUNDRED,
            Percent::from_percent(101)
        )
        .is_none());
        assert!(InterestRate::new(
            Percent::from_percent(101),
            Percent::ZERO,
            Percent::from_percent(101)
        )
        .is_none());
        assert!(InterestRate::new(
            Percent::from_percent(101),
            Percent::from_percent(101),
            Percent::from_percent(101)
        )
        .is_none());
    }

    /// Test suit specifically for verifying correctness of [`InterestRate::calculate`](InterestRate::calculate).cargo fmt
    mod calculate {
        use finance::{
            coin::{Amount, Coin},
            percent::{Percent, Units},
        };

        use crate::{borrow::InterestRate, nlpn::NLpn};

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
            InterestRate::new(
                Percent::from_permille(base_interest_rate),
                Percent::from_permille(utilization_optimal),
                Percent::from_permille(addon_optimal_interest_rate),
            )
            .expect("Rates should be less or equal to a thousand!")
        }

        fn ratio(n: Units, d: Units) -> Percent {
            Percent::from_ratio(n, d)
        }

        #[derive(Copy, Clone)]
        struct InOut((Amount, Amount), (Units, Units));

        fn in_out(InOut((l, b), (n, d)): InOut) -> ((Coin<NLpn>, Coin<NLpn>), Percent) {
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
            for base_rate in 0..=1000 {
                let rate = rate(base_rate, 1000, 0);

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
            for base_rate in 0..=1000 {
                let rate = rate(base_rate, 1000, 1000);

                do_test_calculate(
                    rate,
                    &(1..=1000)
                        .map(move |balance| InOut((0, balance), (base_rate, 1000)))
                        .collect::<Vec<_>>(),
                );
            }
        }

        #[test]
        fn test_corner_set() {
            let rate = rate(1000, 1000, 1000);

            let set = [InOut((0, 0), (1, 1)), InOut((10, 0), (2, 1))];
            do_test_calculate(rate, &set);
        }

        #[test]
        /// Verifies correctness of results against manually calculated, thus verified, set.
        fn test_set_4() {
            let rate = rate(100, 500, 250);

            let set = [
                InOut((10, 1), (554, 1000)),
                InOut((10, 2), (516, 1000)),
                InOut((10, 3), (484, 1000)),
                InOut((10, 4), (457, 1000)),
                InOut((10, 5), (433, 1000)),
                InOut((10, 6), (412, 1000)),
                InOut((10, 7), (394, 1000)),
                InOut((10, 8), (377, 1000)),
                InOut((10, 9), (363, 1000)),
                InOut((10, 10), (35, 100)),
                InOut((10, 11), (338, 1000)),
                InOut((10, 12), (327, 1000)),
                InOut((10, 13), (317, 1000)),
                InOut((10, 14), (308, 1000)),
                InOut((10, 15), (3, 10)),
            ];

            do_test_calculate(rate, &set);
        }
    }
}
