use serde::{Deserialize, Serialize};

use currency::{AnyVisitor, Group, Matcher, MaybeAnyVisitResult, MemberOf};
use sdk::schemars::{self, JsonSchema};

use crate::PaymentGroup;

#[cfg(not(feature = "testing"))]
use self::r#impl as impl_mod;
#[cfg(feature = "testing")]
use self::testing as impl_mod;

#[cfg(not(feature = "testing"))]
mod r#impl;
#[cfg(feature = "testing")]
pub mod testing;

#[derive(Clone, Copy, Debug, PartialEq, Eq, JsonSchema, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct LeaseGroup {}

impl Group for LeaseGroup {
    const DESCR: &'static str = "lease";

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

impl MemberOf<PaymentGroup> for LeaseGroup {}
impl MemberOf<Self> for LeaseGroup {}
