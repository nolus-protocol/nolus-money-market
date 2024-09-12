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

#[derive(
    Clone, Copy, Debug, Ord, PartialEq, PartialOrd, Eq, JsonSchema, Serialize, Deserialize,
)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct LeaseGroup {}

impl Group for LeaseGroup {
    const DESCR: &'static str = "lease";
    type TopG = PaymentGroup;

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

impl MemberOf<PaymentGroup> for LeaseGroup {}
impl MemberOf<Self> for LeaseGroup {}
