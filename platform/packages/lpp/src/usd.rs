use serde::{Deserialize, Serialize};

use currency::{Currency, SymbolStatic};
use sdk::schemars::{self, JsonSchema};

#[derive(
    Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default, Serialize, Deserialize, JsonSchema,
)]
pub struct Usd;
impl Currency for Usd {
    // should not be visible
    const TICKER: SymbolStatic = "USD";
    const BANK_SYMBOL: SymbolStatic = "N/A_N/A_N/A";
    const DEX_SYMBOL: SymbolStatic = "N/A_N/A_N/A";
}
