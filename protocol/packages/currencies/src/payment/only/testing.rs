use currency::{AnyVisitor, Group, Matcher, MaybeAnyVisitResult};

use crate::PaymentGroup;

pub(super) fn maybe_visit<M, V, VisitedG>(
    _matcher: &M,
    visitor: V,
) -> MaybeAnyVisitResult<VisitedG, V>
where
    M: Matcher,
    V: AnyVisitor<VisitedG>,
    VisitedG: Group<TopG = PaymentGroup>,
{
    currency::visit_noone(visitor)
}
