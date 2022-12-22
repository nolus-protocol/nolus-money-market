use serde::{Deserialize, Serialize};

use finance::{
    coin::Coin, currency::Currency, fraction::Fraction, percent::Percent, ratio::Rational,
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

    pub fn calculate<Lpn>(
        &self,
        total_liability_past_quote: Coin<Lpn>,
        total_balance_past_quote: Coin<Lpn>,
    ) -> Percent
    where
        Lpn: Currency,
    {
        // utilization % / utilization_optimal %
        let utilization_rel = Rational::new(
            total_liability_past_quote,
            self.utilization_optimal()
                .of(total_liability_past_quote + total_balance_past_quote),
        );

        self.base_interest_rate()
            + <Rational<Coin<Lpn>> as Fraction<Coin<Lpn>>>::of(
                &utilization_rel,
                self.addon_optimal_interest_rate(),
            )
    }

    fn validate(&self) -> bool {
        self.base_interest_rate <= Percent::HUNDRED
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
