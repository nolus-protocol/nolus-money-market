use std::iter;

use currency::{AnyVisitor, CurrencyDTO, Group, Matcher, MaybeAnyVisitResult};

use super::super::Group as PaymentGroup;

#[inline]
pub(super) fn currencies() -> impl Iterator<Item = CurrencyDTO<super::Group>> {
    iter::empty()
}

#[inline]
pub(super) fn maybe_visit<M, V, VisitedG>(_: &M, visitor: V) -> MaybeAnyVisitResult<VisitedG, V>
where
    M: Matcher,
    V: AnyVisitor<VisitedG>,
    VisitedG: Group<TopG = PaymentGroup>,
{
    currency::visit_noone(visitor)
}
