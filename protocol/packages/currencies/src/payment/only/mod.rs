use currency::{AnyVisitor, Matcher, MaybeAnyVisitResult, MemberOf};

use crate::payment;

#[cfg(not(feature = "testing"))]
pub(crate) mod impl_mod {
    include!(concat!(env!("OUT_DIR"), "/payment_only.rs"));
}

#[cfg(feature = "testing")]
#[path = "testing.rs"]
pub(crate) mod impl_mod;

#[derive(Clone, Copy, Debug, Ord, PartialEq, PartialOrd, Eq)]
pub struct Group {}

impl currency::Group for Group {
    const DESCR: &'static str = "payment only";

    type TopG = payment::Group;

    #[inline]
    fn maybe_visit<M, V>(matcher: &M, visitor: V) -> MaybeAnyVisitResult<Self, V>
    where
        M: Matcher,
        V: AnyVisitor<Self>,
    {
        impl_mod::maybe_visit(matcher, visitor)
    }

    fn maybe_visit_member<M, V>(matcher: &M, visitor: V) -> MaybeAnyVisitResult<Self::TopG, V>
    where
        M: Matcher,
        V: AnyVisitor<Self::TopG>,
    {
        impl_mod::maybe_visit(matcher, visitor)
    }
}

impl MemberOf<Self> for Group {}

impl MemberOf<payment::Group> for Group {}
