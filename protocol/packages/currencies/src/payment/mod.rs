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

    fn maybe_visit<M, V>(matcher: &M, visitor: V) -> MaybeAnyVisitResult<Self, V>
    where
        M: Matcher<Group = Self>,
        V: AnyVisitor<Self, VisitorG = Self>,
    {
        LeaseGroup::maybe_visit_member(&matcher.to_sub_matcher::<LeaseGroup>(), visitor)
            .or_else(|visitor| Lpns::maybe_visit_member(&matcher.to_sub_matcher::<Lpns>(), visitor))
            .or_else(|visitor| {
                Native::maybe_visit_member(&matcher.to_sub_matcher::<Native>(), visitor)
            })
            .or_else(|visitor| {
                PaymentOnlyGroup::maybe_visit_member(
                    &matcher.to_sub_matcher::<PaymentOnlyGroup>(),
                    visitor,
                )
            })
    }

    fn maybe_visit_super_visitor<M, V, TopG>(
        _matcher: &M,
        _visitor: V,
    ) -> MaybeAnyVisitResult<Self, V>
    where
        M: Matcher<Group = Self>,
        V: AnyVisitor<Self, VisitorG = TopG>,
        Self: MemberOf<TopG>,
        TopG: Group,
    {
        unreachable!()
    }

    fn maybe_visit_member<M, V, TopG>(_matcher: &M, _visitor: V) -> MaybeAnyVisitResult<TopG, V>
    where
        M: Matcher<Group = Self>,
        V: AnyVisitor<TopG, VisitorG = TopG>,
        Self: MemberOf<TopG>,
        TopG: Group,
    {
        unreachable!()
    }
}

impl MemberOf<Self> for PaymentGroup {}
