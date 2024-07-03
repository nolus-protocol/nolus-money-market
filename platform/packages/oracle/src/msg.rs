use sdk::schemars::{self, JsonSchema};
use serde::Serialize;

use currency::{CurrencyDTO, Group};

#[derive(Serialize, JsonSchema)]
#[serde(
    deny_unknown_fields,
    rename_all = "snake_case",
    bound(serialize = "", deserialize = "")
)]
pub enum BaseCurrencyQueryMsg<G>
where
    G: Group,
{
    /// Report the base currency as [CurrencyDTO]
    BaseCurrency {},

    /// Provide the price of a currency against the base one
    ///
    /// Return [PriceDTO]
    BasePrice { currency: CurrencyDTO<G> },
}

#[derive(Serialize)]
#[serde(
    deny_unknown_fields,
    rename_all = "snake_case",
    bound(serialize = "", deserialize = "")
)]
pub enum StableCurrencyQueryMsg<G>
where
    G: Group,
{
    /// Report the stable currency as [CurrencyDTO]
    StableCurrency {},

    /// Provide the price of a currency against the stable one
    ///
    /// Return [PriceDTO]
    StablePrice { currency: CurrencyDTO<G> },
}
