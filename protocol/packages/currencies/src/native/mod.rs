use currency::{AnyVisitor, Group, Matcher, MaybeAnyVisitResult, MemberOf};

pub use impl_mod::Nls;

#[cfg(not(feature = "testing"))]
use r#impl as impl_mod;
#[cfg(feature = "testing")]
use testing as impl_mod;

use crate::PaymentGroup;

#[cfg(not(feature = "testing"))]
mod r#impl;
#[cfg(feature = "testing")]
mod testing;

#[derive(Clone, Copy, Debug, Ord, PartialEq, PartialOrd, Eq)]
pub struct Native {}
impl Group for Native {
    const DESCR: &'static str = "native";
    type TopG = PaymentGroup;

    fn maybe_visit<M, V>(matcher: &M, visitor: V) -> MaybeAnyVisitResult<Self, V>
    where
        M: Matcher,
        V: AnyVisitor<Self>,
    {
        currency::maybe_visit_member::<_, Nls, Self, _>(matcher, visitor)
    }

    fn maybe_visit_member<M, V>(matcher: &M, visitor: V) -> MaybeAnyVisitResult<Self::TopG, V>
    where
        M: Matcher,
        V: AnyVisitor<Self::TopG>,
    {
        currency::maybe_visit_member::<_, Nls, Self::TopG, _>(matcher, visitor)
    }
}

impl MemberOf<Self> for Native {}
impl MemberOf<PaymentGroup> for Native {}
