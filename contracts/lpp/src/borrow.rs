use serde::{Deserialize, Serialize};

use finance::percent::Percent;
use sdk::schemars::{self, JsonSchema};

#[derive(Debug, Copy, Clone, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(try_from = "UncheckedInterestRate")]
pub struct InterestRate {
    base_interest_rate: Percent,
    utilization_optimal: Percent,
    addon_optimal_interest_rate: Percent,
}

impl InterestRate {
    pub fn new(
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

    fn validate(&self) -> bool {
        self.base_interest_rate <= Percent::HUNDRED
            && self.utilization_optimal <= Percent::HUNDRED
            && self.addon_optimal_interest_rate <= Percent::HUNDRED
    }
}

impl TryFrom<UncheckedInterestRate> for InterestRate {
    type Error = &'static str;

    fn try_from(value: UncheckedInterestRate) -> Result<Self, Self::Error> {
        Self::new(
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
