use currency::{group::MemberOf, AnyVisitor, Group, Matcher, MaybeAnyVisitResult};

use crate::PaymentGroup;

use self::r#impl;

pub(crate) mod r#impl;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PaymentOnlyGroup {}

impl Group for PaymentOnlyGroup {
    const DESCR: &'static str = "payment only";

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

impl MemberOf<PaymentGroup> for PaymentOnlyGroup {}
impl MemberOf<Self> for PaymentOnlyGroup {}
