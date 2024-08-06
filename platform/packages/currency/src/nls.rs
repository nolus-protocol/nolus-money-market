use serde::{Deserialize, Serialize};

use sdk::schemars::{self, JsonSchema};

use crate::{
    group::MemberOf, AnyVisitor, Currency, Definition, Group, Matcher, MaybeAnyVisitResult,
    SymbolStatic,
};

#[derive(
    Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default, Serialize, Deserialize, JsonSchema,
)]
/// A 'local'-only 'dex-independent' representation of Nls.
///
/// Intended to be used *only* until the TODO below gets done, and *only* in dex-independent usecases:
/// - LP rewards
/// - Relayers' tips
pub struct NlsPlatform;

impl Currency for NlsPlatform {
    type Group = Native;
}

impl Definition for NlsPlatform {
    const TICKER: SymbolStatic = "NLS";

    const BANK_SYMBOL: SymbolStatic = "unls";

    // TODO Define trait PlatformCurrency as a super trait of Currency and
    // merge NlsPlatform and Nls
    const DEX_SYMBOL: SymbolStatic = "N/A_N/A_N/A";

    const DECIMAL_DIGITS: u8 = 6;
}

#[derive(Copy, Clone, Debug, Deserialize, Ord, PartialOrd, PartialEq, Eq)]
pub struct Native {}

impl Group for Native {
    const DESCR: &'static str = "native";

    fn maybe_visit<M, V>(matcher: &M, visitor: V) -> MaybeAnyVisitResult<Self, V>
    where
        M: Matcher<Group = Self>,
        V: AnyVisitor<Self, VisitorG = Self>,
    {
        Self::maybe_visit_member::<_, _, Self>(matcher, visitor)
    }

    fn maybe_visit_super_visitor<M, V, TopG>(
        matcher: &M,
        visitor: V,
    ) -> MaybeAnyVisitResult<Self, V>
    where
        M: Matcher<Group = Self>,
        V: AnyVisitor<Self, VisitorG = TopG>,
        Self: MemberOf<TopG>,
        TopG: Group,
    {
        crate::maybe_visit_member::<_, NlsPlatform, Self, _>(matcher, visitor)
    }

    fn maybe_visit_member<M, V, TopG>(matcher: &M, visitor: V) -> MaybeAnyVisitResult<TopG, V>
    where
        M: Matcher<Group = Self>,
        V: AnyVisitor<TopG, VisitorG = TopG>,
        Self: MemberOf<TopG>,
        TopG: Group,
    {
        crate::maybe_visit_member::<_, NlsPlatform, TopG, _>(matcher, visitor)
    }
}

impl MemberOf<Self> for Native {}
