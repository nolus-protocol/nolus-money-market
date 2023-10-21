use serde::{Deserialize, Serialize};

use currency::SymbolOwned;

#[derive(Serialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub enum QueryMsg {
    /// Provides the oracle configuration
    Config {},
    /// Provides the price of the currency against the base asset
    Price { currency: SymbolOwned },
}

#[derive(Deserialize)]
// we deliberately skip 'deny_unknown_fields' to allow implementations
// include additional data
#[serde(rename_all = "snake_case")]
pub struct Config {
    pub base_asset: SymbolOwned,
}
