use currency::{Currency, SymbolStatic};
use sdk::schemars::{self, JsonSchema};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, JsonSchema)]
pub struct NLpn;
impl Currency for NLpn {
    // should not be visible
    const TICKER: SymbolStatic = "NLpn";
    const BANK_SYMBOL: SymbolStatic = "N/A_N/A_N/A";
    const DEX_SYMBOL: SymbolStatic = "N/A_N/A_N/A";
}
