use currency::{
    AnyVisitor, Group, Matcher, MaybeAnyVisitResult, MaybePairsVisitorResult, MemberOf, PairsGroup,
    PairsVisitor,
};
use sdk::schemars::{self, JsonSchema};
use serde::{Deserialize, Serialize};

use super::{lease::LeaseGroup, lpn::Lpns, native::Native};

pub use self::only::PaymentOnlyGroup;

#[cfg(feature = "testing")]
pub use testing::*;

mod only;
#[cfg(feature = "testing")]
mod testing;

#[derive(
    Clone, Copy, Debug, Ord, PartialEq, PartialOrd, Eq, JsonSchema, Serialize, Deserialize,
)]
pub struct PaymentGroup {}

impl Group for PaymentGroup {
    const DESCR: &'static str = "payment";

    fn maybe_visit<M, V>(matcher: &M, visitor: V) -> MaybeAnyVisitResult<Self, V>
    where
        M: Matcher,
        V: AnyVisitor<Self, VisitorG = Self>,
    {
        LeaseGroup::maybe_visit_member(matcher, visitor)
            .or_else(|visitor| Lpns::maybe_visit_member(matcher, visitor))
            .or_else(|visitor| Native::maybe_visit_member(matcher, visitor))
            .or_else(|visitor| PaymentOnlyGroup::maybe_visit_member(matcher, visitor))
    }

    fn maybe_visit_super_visitor<M, V, TopG>(
        _matcher: &M,
        _visitor: V,
    ) -> MaybeAnyVisitResult<Self, V>
    where
        M: Matcher,
        V: AnyVisitor<Self, VisitorG = TopG>,
        Self: MemberOf<TopG>,
        TopG: Group,
    {
        unreachable!()
    }

    fn maybe_visit_member<M, V, TopG>(_matcher: &M, _visitor: V) -> MaybeAnyVisitResult<TopG, V>
    where
        M: Matcher,
        V: AnyVisitor<TopG, VisitorG = TopG>,
        Self: MemberOf<TopG>,
        TopG: Group,
    {
        unreachable!()
    }
}

impl MemberOf<Self> for PaymentGroup {}

impl PairsGroup for PaymentGroup {
    type CommonGroup = Self;

    fn maybe_visit<M, V>(matcher: &M, visitor: V) -> MaybePairsVisitorResult<V>
    where
        M: Matcher,
        V: PairsVisitor<Pivot = Self, VisitedG = Self::CommonGroup>,
    {
        crate::lease::maybe_visit_buddy(matcher, visitor)
            .or_else(|v| crate::lpn::maybe_visit_buddy(matcher, v))
            .or_else(|v| crate::native::maybe_visit_buddy(matcher, v))
            .or_else(|v| only::maybe_visit_buddy(matcher, v))
    }
}
