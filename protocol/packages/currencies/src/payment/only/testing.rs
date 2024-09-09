use currency::{
    AnyVisitor, Group, Matcher, MaybeAnyVisitResult, MaybePairsVisitorResult, MemberOf,
    PairsVisitor,
};

use crate::PaymentGroup;

pub(super) fn maybe_visit<M, V, TopG>(_matcher: &M, visitor: V) -> MaybeAnyVisitResult<TopG, V>
where
    M: Matcher,
    V: AnyVisitor<TopG>,
    TopG: Group + MemberOf<V::VisitorG>,
{
    currency::visit_noone(visitor)
}

pub(crate) fn maybe_visit_buddy<M, V>(_matcher: &M, visitor: V) -> MaybePairsVisitorResult<V>
where
    M: Matcher,
    V: PairsVisitor<VisitedG = PaymentGroup>,
{
    currency::visit_noone(visitor)
}
