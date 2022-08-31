use finance::currency::SymbolOwned;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::storage::Price;

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
    Below(Price),
    Above(Price),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct Alarm {
    currency: SymbolOwned, // this can be removed if we can take the currency from the Price object
    below: Price,
    above: Option<Price>,
}

impl Alarm {
    pub fn new(currency: SymbolOwned, below: Price, above: Option<Price>) -> Alarm {
        Self {
            currency,
            below,
            above,
        }
    }
    pub fn should_fire(&self, current_price: Price) -> bool {
        current_price.lt(&self.below)
            || (self.above.is_some() && current_price.gt(self.above.as_ref().unwrap()))
    }
}
