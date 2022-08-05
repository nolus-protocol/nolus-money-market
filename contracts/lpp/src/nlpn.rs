use finance::currency::{Currency, SymbolStatic};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(
    Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default, Serialize, Deserialize, JsonSchema,
)]
pub struct NLpn;
impl Currency for NLpn {
    // should not be visible
    const SYMBOL: SymbolStatic = "nlpn";
}
