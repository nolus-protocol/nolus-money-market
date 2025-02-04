use serde::{Deserialize, Serialize};

use sdk::schemars::{self, JsonSchema};

use crate::{
    AnyVisitor, CurrencyDTO, CurrencyDef, Definition, Group, Matcher, MaybeAnyVisitResult,
    MaybePairsVisitorResult, MemberOf, PairsGroup, PairsVisitor,
};

#[derive(
    Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Serialize, Deserialize, JsonSchema,
)]
pub struct Stable();

impl CurrencyDef for Stable {
    type Group = PlatformGroup;

    fn dto() -> &'static CurrencyDTO<Self::Group> {
        const DTO: CurrencyDTO<<Stable as CurrencyDef>::Group> =
            CurrencyDTO::new(&Definition::new("STABLE", "N/A_N/A_N/A", "N/A_N/A_N/A", 0));
        &DTO
    }
}
impl PairsGroup for Stable {
    type CommonGroup = PlatformGroup;

    fn maybe_visit<M, V>(_matcher: &M, _visitor: V) -> MaybePairsVisitorResult<V>
    where
        M: Matcher,
        V: PairsVisitor<Pivot = Self>,
    {
        unreachable!("The 'Stable' platform currency used in pairs resolution!")
    }
}

#[derive(
    Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Serialize, Deserialize, JsonSchema,
)]
/// A 'platform'-only 'dex-independent' representation of Nls.
///
/// Intended to be used *only* until the TODO below gets done, and *only* in dex-independent usecases:
/// - LP rewards
/// - Relayers' tips
pub struct Nls(CurrencyDTO<PlatformGroup>);

impl CurrencyDef for Nls {
    type Group = PlatformGroup;

    fn dto() -> &'static CurrencyDTO<Self::Group> {
        const DTO: CurrencyDTO<<Nls as CurrencyDef>::Group> = CurrencyDTO::new(&Definition::new(
            "NLS",
            "unls",
            // TODO Define trait PlatformCurrency as a super trait of Currency and
            // merge NlsPlatform and Nls
            "N/A_N/A_N/A",
            6,
        ));

        &DTO
    }
}

impl PairsGroup for Nls {
    type CommonGroup = PlatformGroup;

    fn maybe_visit<M, V>(_matcher: &M, _visitor: V) -> MaybePairsVisitorResult<V>
    where
        M: Matcher,
        V: PairsVisitor,
    {
        unreachable!("The 'Nls' platform currency used in pairs resolution!")
    }
}

#[derive(Debug, Copy, Clone, Ord, PartialEq, PartialOrd, Eq, Deserialize)]
pub struct PlatformGroup;
impl Group for PlatformGroup {
    const DESCR: &'static str = "platform currencies";
    type TopG = Self;

    fn maybe_visit<M, V>(matcher: &M, visitor: V) -> MaybeAnyVisitResult<Self, V>
    where
        M: Matcher,
        V: AnyVisitor<Self>,
    {
        Self::maybe_visit_member(matcher, visitor)
    }

    fn maybe_visit_member<M, V>(matcher: &M, visitor: V) -> MaybeAnyVisitResult<Self::TopG, V>
    where
        M: Matcher,
        V: AnyVisitor<Self::TopG>,
    {
        crate::maybe_visit_member::<_, Nls, Self::TopG, _>(matcher, visitor)
            .or_else(|v| MaybeAnyVisitResult::Ok(v.on::<Stable>(Stable::dto())))
        // we accept ANY currency to allow any stable@protocol to be a member
    }
}

impl MemberOf<Self> for PlatformGroup {}
