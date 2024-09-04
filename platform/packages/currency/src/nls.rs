use serde::{Deserialize, Serialize};

use sdk::schemars::{self, JsonSchema};

use crate::{
    group::MemberOf,
    pairs::{MaybePairsVisitorResult, PairsGroup, PairsVisitor},
    AnyVisitor, CurrencyDTO, CurrencyDef, Definition, Group, Matcher, MaybeAnyVisitResult,
};

#[derive(
    Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Serialize, Deserialize, JsonSchema,
)]
/// A 'local'-only 'dex-independent' representation of Nls.
///
/// Intended to be used *only* until the TODO below gets done, and *only* in dex-independent usecases:
/// - LP rewards
/// - Relayers' tips
pub struct NlsPlatform(CurrencyDTO<Native>);

impl CurrencyDef for NlsPlatform {
    type Group = Native;

    fn definition() -> &'static Self {
        const INSTANCE: &NlsPlatform = &NlsPlatform(CurrencyDTO::new(&Definition::new(
            "NLS",
            "unls",
            // TODO Define trait PlatformCurrency as a super trait of Currency and
            // merge NlsPlatform and Nls
            "N/A_N/A_N/A",
            6,
        )));

        INSTANCE
    }

    fn dto(&self) -> &CurrencyDTO<Self::Group> {
        &self.0
    }
}

impl PairsGroup for NlsPlatform {
    type CommonGroup = Native;

    fn maybe_visit<M, V>(_matcher: &M, visitor: V) -> MaybePairsVisitorResult<V>
    where
        M: Matcher,
        V: PairsVisitor,
    {
        crate::visit_noone(visitor)
    }
}

#[derive(Copy, Clone, Debug, Deserialize, Ord, PartialOrd, PartialEq, Eq)]
pub struct Native {}
impl MemberOf<Self> for Native {}
impl Group for Native {
    const DESCR: &'static str = "native";

    fn maybe_visit<M, V>(matcher: &M, visitor: V) -> MaybeAnyVisitResult<Self, V>
    where
        M: Matcher,
        V: AnyVisitor<Self, VisitorG = Self>,
    {
        Self::maybe_visit_member::<_, _, Self>(matcher, visitor)
    }

    fn maybe_visit_super_visitor<M, V, TopG>(
        matcher: &M,
        visitor: V,
    ) -> MaybeAnyVisitResult<Self, V>
    where
        M: Matcher,
        V: AnyVisitor<Self, VisitorG = TopG>,
        Self: MemberOf<TopG>,
        TopG: Group,
    {
        crate::maybe_visit_member::<_, NlsPlatform, Self, _>(matcher, visitor)
    }

    fn maybe_visit_member<M, V, TopG>(matcher: &M, visitor: V) -> MaybeAnyVisitResult<TopG, V>
    where
        M: Matcher,
        V: AnyVisitor<TopG, VisitorG = TopG>,
        Self: MemberOf<TopG>,
        TopG: Group,
    {
        crate::maybe_visit_member::<_, NlsPlatform, TopG, _>(matcher, visitor)
    }
}
