use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use finance::{currency::SymbolOwned, price::PriceDTO};

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
    currency: SymbolOwned, // this can be removed if we can take the currency from the Price object
    below: PriceDTO,
    above: Option<PriceDTO>,
}

impl Alarm {
    pub fn new<P>(currency: SymbolOwned, below: P, above: Option<P>) -> Alarm
    where
        P: Into<PriceDTO>,
    {
        Self {
            currency,
            below: below.into(),
            above: above.map(Into::into),
        }
    }

    pub fn should_fire(&self, current_price: PriceDTO) -> bool {
        current_price.lt(&self.below)
            || (self.above.is_some() && current_price.gt(self.above.as_ref().unwrap()))
    }
}
