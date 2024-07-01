use currency::{group::MemberOf, AnyVisitor, Group, Matcher, MaybeAnyVisitResult};

use super::{lease::LeaseGroup, lpn::Lpns, native::Native};

pub use self::only::PaymentOnlyGroup;

mod only;
mod osmosis_tests;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PaymentGroup {}

impl Group for PaymentGroup {
    const DESCR: &'static str = "payment";

    fn maybe_visit<M, V>(matcher: &M, visitor: V) -> MaybeAnyVisitResult<V>
    where
        M: Matcher + ?Sized,
        V: AnyVisitor<VisitedG = Self>,
    {
        LeaseGroup::maybe_visit_member(matcher, visitor)
            .or_else(|visitor| Lpns::maybe_visit_member(matcher, visitor))
            .or_else(|visitor| Native::maybe_visit_member(matcher, visitor))
            .or_else(|visitor| PaymentOnlyGroup::maybe_visit_member(matcher, visitor))
    }

    fn maybe_visit_member<M, V>(_matcher: &M, _visitor: V) -> MaybeAnyVisitResult<V>
    where
        M: Matcher + ?Sized,
        V: AnyVisitor,
        Self: MemberOf<V::VisitedG>,
    {
        unreachable!()
    }
}

impl MemberOf<Self> for PaymentGroup {}
