use serde::{Deserialize, Serialize};

use sdk::schemars::{self, JsonSchema};

use crate::SpotPrice;

pub mod errors;
pub mod price;

pub type Id = u64;

#[derive(Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteAlarmMsg {
    PriceAlarm(Alarm),
}

#[derive(Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
#[cfg_attr(test, derive(Debug))]
#[serde(rename_all = "snake_case")]
pub enum Event {
    Below(SpotPrice),
    Above(SpotPrice),
}

#[derive(Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
#[cfg_attr(any(test, feature = "testing"), derive(Debug, Clone))]
pub struct Alarm {
    below: SpotPrice,
    above: Option<SpotPrice>,
}

impl Alarm {
    pub fn new<P>(below: P, above: Option<P>) -> Alarm
    where
        P: Into<SpotPrice>,
    {
        let below = below.into();
        let above = above.map(Into::into);
        debug_assert!(
            above.is_none()
                || above.as_ref().map(|price| price.base().ticker()) == Some(below.base().ticker())
        );
        Self { below, above }
    }

    pub fn should_fire(&self, current_price: &SpotPrice) -> bool {
        current_price < &self.below
            || (self.above.is_some() && current_price > self.above.as_ref().unwrap())
    }
}
