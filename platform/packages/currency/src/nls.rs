use serde::{Deserialize, Serialize};

use sdk::schemars::{self, JsonSchema};

use crate::currency::{Currency, SymbolStatic};

#[derive(
    Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default, Serialize, Deserialize, JsonSchema,
)]
pub struct NlsPlatform;
impl Currency for NlsPlatform {
    const TICKER: SymbolStatic = "NLS";
    const BANK_SYMBOL: SymbolStatic = "unls";
    // TODO Define trait PlatformCurrency as a super trait of Currency and
    // merge NlsPlatform and Nls
    const DEX_SYMBOL: SymbolStatic = "N/A_N/A_N/A";
}
