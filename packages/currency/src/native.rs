use serde::{Deserialize, Serialize};

use finance::currency::{Currency, SymbolStatic};
use sdk::schemars::{self, JsonSchema};

#[derive(
    Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default, Serialize, Deserialize, JsonSchema,
)]
pub struct Nls;
impl Currency for Nls {
    const TICKER: SymbolStatic = "NLS";
    const BANK_SYMBOL: SymbolStatic = "unls";
}
