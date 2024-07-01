use serde::{Deserialize, Serialize};

use currency::{
    AnyVisitor, Currency, Definition, Group, MaybeAnyVisitResult, SymbolMatcher, SymbolSlice,
    SymbolStatic,
};
use finance::coin::Coin;
use sdk::schemars::{self, JsonSchema};

#[derive(
    Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default, Serialize, Deserialize, JsonSchema,
)]
pub struct Stable;

impl Currency for Stable {
    type Group = StableCurrencyGroup;
}
impl Definition for Stable {
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
        Self::maybe_visit_member(Imatcher, visitor)
    }
    
    fn maybe_visit_member<M, V>(matcher: &M, visitor: V) -> MaybeAnyVisitResult<V>
    where
        M: currency::Matcher + ?Sized,
        V: AnyVisitor,
        Self: MemberOf<V::VisitedG> {
            Ok(visitor.on::<Stable>())
    }
}
