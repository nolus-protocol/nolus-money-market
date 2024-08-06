use serde::{Deserialize, Serialize};

use currency::{
    AnyVisitor, Currency, Definition, Group, Matcher, MaybeAnyVisitResult, MemberOf, SymbolStatic,
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

#[derive(Debug, Copy, Clone, Ord, PartialEq, PartialOrd, Eq, Deserialize)]
pub struct StableCurrencyGroup;
impl Group for StableCurrencyGroup {
    const DESCR: &'static str = "stable currency";

    fn maybe_visit<M, V>(matcher: &M, visitor: V) -> MaybeAnyVisitResult<Self, V>
    where
        M: Matcher<Group = Self>,
        V: AnyVisitor<Self, VisitorG = Self>,
    {
        Self::maybe_visit_member(matcher, visitor)
    }

    fn maybe_visit_super_visitor<M, V, TopG>(
        _matcher: &M,
        _visitor: V,
    ) -> MaybeAnyVisitResult<Self, V>
    where
        M: Matcher<Group = Self>,
        V: AnyVisitor<Self, VisitorG = TopG>,
        Self: MemberOf<TopG>,
        TopG: Group,
    {
        unreachable!("There is no parent group to this one!")
    }

    fn maybe_visit_member<M, V, TopG>(_matcher: &M, visitor: V) -> MaybeAnyVisitResult<TopG, V>
    where
        M: Matcher<Group = Self>,
        V: AnyVisitor<TopG, VisitorG = TopG>,
        Self: MemberOf<TopG>,
        TopG: Group,
    {
        MaybeAnyVisitResult::Ok(visitor.on::<Stable>()) // we accept ANY currency to allow any stable@protocol to be a member
    }
}

impl MemberOf<Self> for StableCurrencyGroup {}
