use currency::{AnyVisitor, Group, Matcher, MaybeAnyVisitResult, MemberOf};
use sdk::schemars::{self, JsonSchema};
use serde::{Deserialize, Serialize};

use super::{lease::LeaseGroup, lpn::Lpns, native::Native};

pub use self::only::PaymentOnlyGroup;

#[cfg(feature = "testing")]
pub use testing::*;

mod only;
#[cfg(feature = "testing")]
mod testing;

#[derive(Clone, Copy, Debug, PartialEq, Eq, JsonSchema, Serialize, Deserialize)]
pub struct PaymentGroup {}

impl Group for PaymentGroup {
    const DESCR: &'static str = "payment";

    fn maybe_visit<M, V>(matcher: &M, visitor: V) -> MaybeAnyVisitResult<V>
    where
        M: Matcher<Group = Self>,
        V: AnyVisitor<VisitedG = Self>,
    {
        LeaseGroup::maybe_visit_member(matcher, visitor)
            .or_else(|visitor| Lpns::maybe_visit_member(matcher, visitor))
            .or_else(|visitor| Native::maybe_visit_member(matcher, visitor))
            .or_else(|visitor| PaymentOnlyGroup::maybe_visit_member(matcher, visitor))
    }

    fn maybe_visit_member<M, V>(_matcher: &M, _visitor: V) -> MaybeAnyVisitResult<V>
    where
        M: Matcher,
        V: AnyVisitor,
        Self: MemberOf<V::VisitedG> + MemberOf<M::Group>,
    {
        unreachable!()
    }
}

impl MemberOf<Self> for PaymentGroup {}
