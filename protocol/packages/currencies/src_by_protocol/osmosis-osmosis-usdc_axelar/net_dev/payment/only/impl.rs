use currency::{AnyVisitor, Group, Matcher, MaybeAnyVisitResult, MemberOf};

use crate::{PaymentGroup, PaymentOnlyGroup};

pub(super) fn maybe_visit<M, V, VisitedG>(
    _matcher: &M,
    visitor: V,
) -> MaybeAnyVisitResult<VisitedG, V>
where
    M: Matcher,
    V: AnyVisitor<VisitedG>,
    PaymentOnlyGroup: MemberOf<VisitedG>,
    VisitedG: Group<TopG = PaymentGroup>,
{
    currency::visit_noone(visitor)
}
