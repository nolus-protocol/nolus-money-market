use serde::{Deserialize, Serialize};

use currency::{
    group::MemberOf, AnyVisitor, Currency, Definition, Group, Matcher, MaybeAnyVisitResult,
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

#[derive(Debug, Copy, Clone, PartialEq, Eq, Deserialize)]
pub struct StableCurrencyGroup;
impl Group for StableCurrencyGroup {
    const DESCR: &'static str = "stable currency group";

    fn maybe_visit<M, V>(matcher: &M, visitor: V) -> MaybeAnyVisitResult<V>
    where
        M: Matcher + ?Sized,
        V: AnyVisitor<VisitedG = Self>,
    {
        Self::maybe_visit_member(matcher, visitor)
    }

    fn maybe_visit_member<M, V>(matcher: &M, visitor: V) -> MaybeAnyVisitResult<V>
    where
        M: Matcher + ?Sized,
        V: AnyVisitor,
        Self: MemberOf<V::VisitedG>,
    {
        currency::maybe_visit_any::<_, Stable, _>(matcher, visitor)
    }
}

impl MemberOf<Self> for StableCurrencyGroup {}
