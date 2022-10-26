use serde::{Deserialize, Serialize};

use finance::price::dto::PriceDTO;
use sdk::schemars::{self, JsonSchema};

pub mod errors;
pub mod price;

pub type Id = u64;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteAlarmMsg {
    PriceAlarm(Alarm),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Event {
    Below(PriceDTO),
    Above(PriceDTO),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct Alarm {
    below: PriceDTO,
    above: Option<PriceDTO>,
}

impl Alarm {
    pub fn new<P>(below: P, above: Option<P>) -> Alarm
    where
        P: Into<PriceDTO>,
    {
        let below = below.into();
        let above = above.map(Into::into);
        debug_assert!(
            above.is_none()
                || above.as_ref().map(|price| price.base().ticker()) == Some(below.base().ticker())
        );
        Self { below, above }
    }

    pub fn should_fire(&self, current_price: PriceDTO) -> bool {
        current_price.lt(&self.below)
            || (self.above.is_some() && current_price.gt(self.above.as_ref().unwrap()))
    }
}
