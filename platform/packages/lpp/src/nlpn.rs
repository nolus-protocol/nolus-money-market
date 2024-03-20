use serde::{Deserialize, Serialize};

use currency::{Currency, SymbolStatic};
use sdk::schemars::{self, JsonSchema};

#[derive(
    Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default, Serialize, Deserialize, JsonSchema,
)]
pub struct NLpn;
impl Currency for NLpn {
    // TODO the Currency abstraction has since evolved. Remove this impl.
    const TICKER: SymbolStatic = "NLpn";
    const BANK_SYMBOL: SymbolStatic = "N/A_N/A_N/A";
    const DEX_SYMBOL: SymbolStatic = "N/A_N/A_N/A";
}
