use serde::{Deserialize, Serialize};

use currency::{AnyVisitor, Group, Matcher, MaybeAnyVisitResult, MemberOf};
use sdk::schemars::{self, JsonSchema};

use crate::PaymentGroup;

#[cfg(not(feature = "testing"))]
pub(crate) use self::r#impl as impl_mod;
#[cfg(feature = "testing")]
pub(crate) use self::testing as impl_mod;

#[cfg(not(feature = "testing"))]
pub(crate) mod r#impl;
#[cfg(feature = "testing")]
pub(crate) mod testing;

pub(crate) use impl_mod::maybe_visit_buddy;

#[derive(
    Clone, Copy, Debug, Ord, PartialEq, PartialOrd, Eq, JsonSchema, Serialize, Deserialize,
)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct LeaseGroup {}

impl Group for LeaseGroup {
    const DESCR: &'static str = "lease";

    fn maybe_visit<M, V>(matcher: &M, visitor: V) -> MaybeAnyVisitResult<Self, V>
    where
        M: Matcher,
        V: AnyVisitor<Self, VisitorG = Self>,
    {
        Self::maybe_visit_member(matcher, visitor)
    }

    fn maybe_visit_super_visitor<M, V, TopG>(
        matcher: &M,
        visitor: V,
    ) -> MaybeAnyVisitResult<Self, V>
    where
        M: Matcher,
        V: AnyVisitor<Self, VisitorG = TopG>,
        Self: MemberOf<TopG>,
        TopG: Group,
    {
        impl_mod::maybe_visit::<_, _, Self>(matcher, visitor)
    }

    fn maybe_visit_member<M, V, TopG>(matcher: &M, visitor: V) -> MaybeAnyVisitResult<TopG, V>
    where
        M: Matcher,
        V: AnyVisitor<TopG, VisitorG = TopG>,
        Self: MemberOf<TopG>,
        TopG: Group,
    {
        impl_mod::maybe_visit(matcher, visitor)
    }
}

impl MemberOf<PaymentGroup> for LeaseGroup {}
impl MemberOf<Self> for LeaseGroup {}
