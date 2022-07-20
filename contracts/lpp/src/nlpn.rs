use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use finance::currency::{Currency, SymbolStatic};

#[derive(
    Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default, Serialize, Deserialize, JsonSchema,
)]
pub struct NLpn;
impl Currency for NLpn {
    // should not be visible
    const SYMBOL: SymbolStatic = "nlpn";
}
