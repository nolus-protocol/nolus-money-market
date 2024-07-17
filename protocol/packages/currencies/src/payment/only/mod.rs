use currency::{AnyVisitor, Group, Matcher, MaybeAnyVisitResult, MemberOf};

#[cfg(not(feature = "testing"))]
use r#impl as impl_mod;
#[cfg(feature = "testing")]
use testing as impl_mod;

use crate::PaymentGroup;

#[cfg(not(feature = "testing"))]
mod r#impl;
#[cfg(feature = "testing")]
mod testing;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PaymentOnlyGroup {}

impl Group for PaymentOnlyGroup {
    const DESCR: &'static str = "payment only";

    fn maybe_visit<M, V>(matcher: &M, visitor: V) -> MaybeAnyVisitResult<V>
    where
        M: Matcher<Group = Self>,
        V: AnyVisitor<VisitedG = Self>,
    {
        Self::maybe_visit_member(matcher, visitor)
    }

    fn maybe_visit_member<M, V>(matcher: &M, visitor: V) -> MaybeAnyVisitResult<V>
    where
        M: Matcher,
        V: AnyVisitor,
        Self: MemberOf<V::VisitedG> + MemberOf<M::Group>,
    {
        impl_mod::maybe_visit(matcher, visitor)
    }
}

impl MemberOf<PaymentGroup> for PaymentOnlyGroup {}
impl MemberOf<Self> for PaymentOnlyGroup {}
