use sdk::schemars::{self, JsonSchema};
use serde::Serialize;

use currency::SymbolOwned;

#[derive(Serialize, Clone, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub enum QueryMsg {
    /// Report the base currency as [SymbolOwned]
    BaseCurrency {},

    /// Provides the price of a currency against the base one
    ///
    /// Return [PriceDTO]
    BasePrice { currency: SymbolOwned },
}
