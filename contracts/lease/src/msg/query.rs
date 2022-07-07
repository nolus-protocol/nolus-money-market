use finance::{coin::Coin, percent::Percent, currency::Currency};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct StateQuery {}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum StateResponse<C, Lpn>
where
    C: Currency,
    Lpn: Currency,
{
    Opened {
        amount: Coin<C>,
        interest_rate: Percent,
        principal_due: Coin<Lpn>,
        interest_due: Coin<Lpn>,
    },
    Paid(Coin<C>),
    Closed(),
}
