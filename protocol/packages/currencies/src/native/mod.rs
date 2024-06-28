use currency::{group::MemberOf, AnyVisitor, Group, Matcher, MaybeAnyVisitResult};

use crate::PaymentGroup;

pub(crate) mod r#impl;

pub type Nls = r#impl::Nls;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Native {}
impl Group for Native {
    const DESCR: &'static str = "native";

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
        currency::maybe_visit_any::<_, Nls, _>(matcher, visitor)
    }
}

impl MemberOf<Self> for Native {}
impl MemberOf<PaymentGroup> for Native {}
