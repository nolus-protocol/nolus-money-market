use serde::{Deserialize, Serialize};

use currency::{AnyVisitor, Currency, Group, Matcher, MaybeAnyVisitResult, MemberOf, SymbolStatic};
use finance::coin::Coin;
use sdk::schemars::{self, JsonSchema};

#[derive(
    Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default, Serialize, Deserialize, JsonSchema,
)]
pub struct Stable;

impl Currency for Stable {
    type Group = StableCurrencyGroup;

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
    const DESCR: &'static str = "stable currency";

    fn maybe_visit<M, V>(matcher: &M, visitor: V) -> MaybeAnyVisitResult<V>
    where
        M: Matcher<Group = Self>,
        V: AnyVisitor<VisitedG = Self>,
    {
        Self::maybe_visit_member(matcher, visitor)
    }

    fn maybe_visit_member<M, V>(_matcher: &M, visitor: V) -> MaybeAnyVisitResult<V>
    where
        M: Matcher,
        V: AnyVisitor,
        Self: MemberOf<V::VisitedG> + MemberOf<M::Group>,
    {
        MaybeAnyVisitResult::Ok(visitor.on::<Stable>()) // we accept ANY currency to allow any stable@protocol to be a member
    }
}

impl MemberOf<Self> for StableCurrencyGroup {}
