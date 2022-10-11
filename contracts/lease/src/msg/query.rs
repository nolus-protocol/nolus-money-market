use serde::{Deserialize, Serialize};

use finance::{coin::Coin, currency::Currency, percent::Percent};
use sdk::{
    cosmwasm_std::Timestamp,
    schemars::{self, JsonSchema},
};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct StateQuery {}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum StateResponse<Asset, Lpn>
where
    Asset: Currency,
    Lpn: Currency,
{
    Opened {
        amount: Coin<Asset>,
        interest_rate: Percent,
        interest_rate_margin: Percent,
        principal_due: Coin<Lpn>,
        previous_margin_due: Coin<Lpn>,
        previous_interest_due: Coin<Lpn>,
        current_margin_due: Coin<Lpn>,
        current_interest_due: Coin<Lpn>,
        validity: Timestamp,
    },
    Paid(Coin<Asset>),
    Closed(),
}
