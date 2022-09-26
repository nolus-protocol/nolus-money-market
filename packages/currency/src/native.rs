use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use finance::currency::{Currency, SymbolStatic};

#[derive(
    Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default, Serialize, Deserialize, JsonSchema,
)]
pub struct Nls;
impl Currency for Nls {
    const SYMBOL: SymbolStatic = "unls";
}
