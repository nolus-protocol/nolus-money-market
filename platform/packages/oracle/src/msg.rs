use sdk::schemars::{self, JsonSchema};
use serde::Serialize;

use currency::{CurrencyDTO, SymbolOwned};

#[derive(Serialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub enum BaseCurrencyQueryMsg {
    /// Report the base currency as [CurrencyDTO]
    BaseCurrency {},

    /// Provide the price of a currency against the base one
    ///
    /// Return [PriceDTO]
    BasePrice { currency: CurrencyDTO<G> },
}

#[derive(Serialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub enum StableCurrencyQueryMsg {
    /// Report the stable currency as [SymbolOwned]
    StableCurrency {},

    /// Provide the price of a currency against the stable one
    ///
    /// Return [PriceDTO]
    StablePrice { currency: SymbolOwned },
}
