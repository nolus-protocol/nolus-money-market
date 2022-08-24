use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use finance::{
    coin::Coin,
    currency::Currency,
    percent::Percent
};

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum LiquidationStatus<Lpn>
where
    Lpn: Currency,
{
    None,
    FirstWarning(Percent),
    SecondWarning(Percent),
    ThirdWarning(Percent),
    PartialLiquidation(Coin<Lpn>),
    FullLiquidation(Coin<Lpn>),
}
