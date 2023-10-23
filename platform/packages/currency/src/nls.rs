use serde::{Deserialize, Serialize};

use sdk::schemars::{self, JsonSchema};

use crate::currency::{Currency, SymbolStatic};

#[derive(
    Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default, Serialize, Deserialize, JsonSchema,
)]
/// A 'local'-only 'dex-independent' representation of Nls.
///
/// Intended to be used *only* until the TODO below gets done, and *only* in dex-independent usecases:
/// - LP rewards
/// - Relayers' tips
pub struct NlsPlatform;
impl Currency for NlsPlatform {
    const TICKER: SymbolStatic = "NLS";
    const BANK_SYMBOL: SymbolStatic = "unls";
    // TODO Define trait PlatformCurrency as a super trait of Currency and
    // merge NlsPlatform and Nls
    const DEX_SYMBOL: SymbolStatic = "N/A_N/A_N/A";
}
