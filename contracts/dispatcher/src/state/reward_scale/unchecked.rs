use serde::{Deserialize, Serialize};

use sdk::schemars::{self, JsonSchema};

use super::Bar;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct RewardScale {
    bars: Vec<Bar>,
}

impl From<super::RewardScale> for RewardScale {
    fn from(reward_scale: super::RewardScale) -> Self {
        Self {
            bars: reward_scale.bars,
        }
    }
}

impl TryFrom<RewardScale> for super::RewardScale {
    type Error = <Self as TryFrom<Vec<Bar>>>::Error;

    fn try_from(reward_scale: RewardScale) -> Result<Self, Self::Error> {
        super::RewardScale::try_from(reward_scale.bars)
    }
}
