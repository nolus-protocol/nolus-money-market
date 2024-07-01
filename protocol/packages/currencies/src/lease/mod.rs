use currency::{group::MemberOf, AnyVisitor, Group, Matcher, MaybeAnyVisitResult};

use crate::PaymentGroup;

use self::r#impl as impl_mod;

pub(crate) mod r#impl;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LeaseGroup {}

impl Group for LeaseGroup {
    const DESCR: &'static str = "lease";

    fn maybe_visit<M, V>(matcher: &M, visitor: V) -> MaybeAnyVisitResult<V>
    where
        M: Matcher + ?Sized,
        V: AnyVisitor<VisitedG = Self>,
    {
        Self::maybe_visit_member(matcher, visitor)
    }

    fn maybe_visit_member<M, V>(matcher: &M, visitor: V) -> MaybeAnyVisitResult<V>
    where
        M: Matcher + ?Sized,
        V: AnyVisitor,
        Self: MemberOf<V::VisitedG>,
    {
        impl_mod::maybe_visit(matcher, visitor)
    }
}

impl MemberOf<PaymentGroup> for LeaseGroup {}
impl MemberOf<Self> for LeaseGroup {}
