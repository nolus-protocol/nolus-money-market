use serde::Serialize;

use currency::SymbolOwned;

#[derive(Serialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub enum QueryMsg {
    /// Report the stable currency as [SymbolOwned]
    StableCurrency {},
    /// Provide the price of a currency against the stable one
    StablePrice { currency: SymbolOwned },
}
