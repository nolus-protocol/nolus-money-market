use currency::{
    AnyVisitor, CurrencyDef, Group, InPoolWith, Matcher, MaybeAnyVisitResult,
    MaybePairsVisitorResult, MemberOf, PairsVisitor,
};

pub use impl_mod::Nls;

#[cfg(not(feature = "testing"))]
use r#impl as impl_mod;
#[cfg(feature = "testing")]
use testing as impl_mod;

use crate::PaymentGroup;

#[cfg(not(feature = "testing"))]
mod r#impl;
#[cfg(feature = "testing")]
mod testing;

#[derive(Clone, Copy, Debug, Ord, PartialEq, PartialOrd, Eq)]
pub struct Native {}
impl Group for Native {
    const DESCR: &'static str = "native";

    fn maybe_visit<M, V>(matcher: &M, visitor: V) -> MaybeAnyVisitResult<Self, V>
    where
        M: Matcher,
        V: AnyVisitor<Self, VisitorG = Self>,
    {
        Self::maybe_visit_member(matcher, visitor)
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
        currency::maybe_visit_member::<_, Nls, Self, _>(matcher, visitor)
    }

    fn maybe_visit_member<M, V, TopG>(matcher: &M, visitor: V) -> MaybeAnyVisitResult<TopG, V>
    where
        M: Matcher,
        V: AnyVisitor<TopG, VisitorG = TopG>,
        Self: MemberOf<TopG>,
        TopG: Group,
    {
        currency::maybe_visit_member::<_, Nls, TopG, _>(matcher, visitor)
    }
}

pub(crate) fn maybe_visit_buddy<M, V>(matcher: &M, visitor: V) -> MaybePairsVisitorResult<V>
where
    M: Matcher,
    V: PairsVisitor<Pivot = PaymentGroup, VisitedG = PaymentGroup>,
{
    use currency::maybe_visit_buddy as maybe_visit;
    maybe_visit::<Nls, _, _>(Nls::definition().dto(), matcher, visitor)
}

impl MemberOf<Self> for Native {}
impl MemberOf<PaymentGroup> for Native {}

impl InPoolWith<PaymentGroup> for Nls {}
