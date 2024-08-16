use currency::{AnyVisitor, Group, Matcher, MaybeAnyVisitResult, MemberOf};

use crate::PaymentOnlyGroup;

pub(super) fn maybe_visit<M, V, TopG>(_matcher: &M, visitor: V) -> MaybeAnyVisitResult<TopG, V>
where
    M: Matcher,
    V: AnyVisitor<TopG>,
    PaymentOnlyGroup: MemberOf<TopG> + MemberOf<V::VisitorG>,
    TopG: Group + MemberOf<V::VisitorG>,
{
    currency::visit_noone(visitor)
}
