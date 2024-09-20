use serde::{Deserialize, Serialize};

use currency::{AnyVisitor, Matcher, MaybeAnyVisitResult, MemberOf};
use sdk::schemars::JsonSchema;

use crate::{lease, lpn, native};

pub use self::only::Group as OnlyGroup;
#[cfg(feature = "testing")]
pub use self::testing::*;

pub(crate) mod only;
#[cfg(feature = "testing")]
mod testing;

#[derive(
    Clone, Copy, Debug, Ord, PartialEq, PartialOrd, Eq, Serialize, Deserialize, JsonSchema,
)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
#[schemars(crate = "sdk::schemars")]
pub enum Group {}

impl currency::Group for Group {
    const DESCR: &'static str = "payment";

    type TopG = Self;

    #[inline]
    fn maybe_visit<M, V>(matcher: &M, visitor: V) -> MaybeAnyVisitResult<Self, V>
    where
        M: Matcher,
        V: AnyVisitor<Self>,
    {
        lease::Group::maybe_visit_member(matcher, visitor)
            .or_else(|visitor| lpn::Group::maybe_visit_member(matcher, visitor))
            .or_else(|visitor| native::Group::maybe_visit_member(matcher, visitor))
            .or_else(|visitor| only::Group::maybe_visit_member(matcher, visitor))
    }

    #[cold]
    #[inline]
    fn maybe_visit_member<M, V>(_: &M, _: V) -> MaybeAnyVisitResult<Self::TopG, V>
    where
        M: Matcher,
        V: AnyVisitor<Self::TopG>,
    {
        unimplemented!()
    }
}

impl MemberOf<Self> for Group {}
