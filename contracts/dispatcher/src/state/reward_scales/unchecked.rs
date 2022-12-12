use serde::{Deserialize, Serialize};

use sdk::schemars::{self, JsonSchema};

use super::RewardScale;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct RewardScales {
    scales: Vec<RewardScale>,
}

impl From<super::RewardScales> for RewardScales {
    fn from(value: super::RewardScales) -> Self {
        Self {
            scales: value.scales,
        }
    }
}

impl TryFrom<RewardScales> for super::RewardScales {
    type Error = <Self as TryFrom<Vec<RewardScale>>>::Error;

    fn try_from(reward_scales: RewardScales) -> Result<Self, Self::Error> {
        super::RewardScales::try_from(reward_scales.scales)
    }
}
