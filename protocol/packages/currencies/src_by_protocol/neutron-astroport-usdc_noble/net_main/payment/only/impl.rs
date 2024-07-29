use currency::{AnyVisitor, Group, Matcher, MaybeAnyVisitResult};

use crate::PaymentOnlyGroup;

pub(super) fn maybe_visit<M, V, TopG>(matcher: &M, visitor: V) -> MaybeAnyVisitResult<TopG, V>
where
    M: Matcher<Group = PaymentOnlyGroup>,
    V: AnyVisitor<TopG>,
    LeaseGroup: MemberOf<TopG> + MemberOf<V::VisitorG>,
    TopG: Group + MemberOf<V::VisitorG>,
{
    currency::visit_noone(visitor)
}
