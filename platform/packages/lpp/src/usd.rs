use finance::coin::Coin;
use serde::{Deserialize, Serialize};

use currency::{
    AnyVisitor, Currency, Group, Matcher, MaybeAnyVisitResult, SymbolSlice, SymbolStatic,
};
use sdk::schemars::{self, JsonSchema};

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
pub struct Usd;
impl Currency for Usd {
    // should not be visible
    const TICKER: SymbolStatic = "USD";
    const BANK_SYMBOL: SymbolStatic = "N/A_N/A_N/A";
    const DEX_SYMBOL: SymbolStatic = "N/A_N/A_N/A";
}

pub type CoinUsd = Coin<Usd>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, JsonSchema)]
pub struct UsdGroup;
impl Group for UsdGroup {
    const DESCR: &'static str = "usd group";

    fn maybe_visit<M, V>(_matcher: &M, _symbol: &SymbolSlice, visitor: V) -> MaybeAnyVisitResult<V>
    where
        M: Matcher + ?Sized,
        V: AnyVisitor,
    {
        Ok(visitor.on::<Usd>())
    }
}
