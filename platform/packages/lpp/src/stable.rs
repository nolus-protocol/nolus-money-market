use serde::{Deserialize, Serialize};

use currency::{
    AnyVisitor, Currency, Group, SymbolMatcher, MaybeAnyVisitResult, SymbolSlice, SymbolStatic,
};
use finance::coin::Coin;
use sdk::schemars::{self, JsonSchema};

#[derive(
    Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default, Serialize, Deserialize, JsonSchema,
)]
pub struct Stable;

impl Currency for Stable {
    // should not be visible
    const TICKER: SymbolStatic = "STABLE";

    const BANK_SYMBOL: SymbolStatic = "N/A_N/A_N/A";

    const DEX_SYMBOL: SymbolStatic = "N/A_N/A_N/A";

    const DECIMAL_DIGITS: u8 = 6;
}

pub type CoinStable = Coin<Stable>;

#[derive(PartialEq, Eq, Deserialize)]
pub struct StableCurrencyGroup;
impl Group for StableCurrencyGroup {
    const DESCR: &'static str = "stable currency group";

    fn maybe_visit<M, V>(_matcher: &M, _symbol: &SymbolSlice, visitor: V) -> MaybeAnyVisitResult<V>
    where
        M: SymbolMatcher + ?Sized,
        V: AnyVisitor,
    {
        Ok(visitor.on::<Stable>())
    }
}
