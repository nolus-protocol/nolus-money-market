use serde::Serialize;

use currency::SymbolOwned;

#[derive(Serialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub enum QueryMsg {
    /// Report the stable currency as [SymbolOwned]
    BaseCurrency {},
    /// Provides the price of the provided currency against the stable one
    BasePrice { currency: SymbolOwned },
}
