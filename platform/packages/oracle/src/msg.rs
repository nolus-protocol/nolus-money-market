use serde::Serialize;

use currency::SymbolOwned;

#[derive(Serialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub enum QueryMsg {
    /// Report the base currency as [SymbolOwned]
    BaseCurrency {},
    /// Provides the price of the currency against the base asset
    Price { currency: SymbolOwned },
}
