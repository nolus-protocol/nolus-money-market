use currency::{AnyVisitor, Group, Matcher, MaybeAnyVisitResult, MemberOf};

#[cfg(not(feature = "testing"))]
pub(crate) use r#impl as impl_mod;
#[cfg(feature = "testing")]
pub(crate) use testing as impl_mod;

use crate::PaymentGroup;

#[cfg(not(feature = "testing"))]
pub(crate) mod r#impl;
#[cfg(feature = "testing")]
pub(crate) mod testing;

#[derive(Clone, Copy, Debug, Ord, PartialEq, PartialOrd, Eq)]
pub struct PaymentOnlyGroup {}

impl Group for PaymentOnlyGroup {
    const DESCR: &'static str = "payment only";
    type TopG = PaymentGroup;

    fn maybe_visit<M, V>(matcher: &M, visitor: V) -> MaybeAnyVisitResult<Self, V>
    where
        M: Matcher,
        V: AnyVisitor<Self>,
        Self: Group<TopG = Self>,
    {
        Self::maybe_visit_member(matcher, visitor)
    }

    fn maybe_visit_super_visitor<M, V>(matcher: &M, visitor: V) -> MaybeAnyVisitResult<Self, V>
    where
        M: Matcher,
        V: AnyVisitor<Self>,
    {
        impl_mod::maybe_visit::<_, _, Self>(matcher, visitor)
    }

    fn maybe_visit_member<M, V>(matcher: &M, visitor: V) -> MaybeAnyVisitResult<Self::TopG, V>
    where
        M: Matcher,
        V: AnyVisitor<Self::TopG>,
    {
        impl_mod::maybe_visit(matcher, visitor)
    }
}

impl MemberOf<PaymentGroup> for PaymentOnlyGroup {}
impl MemberOf<Self> for PaymentOnlyGroup {}
