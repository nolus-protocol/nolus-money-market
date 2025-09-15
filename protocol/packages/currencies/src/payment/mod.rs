use std::{borrow::Borrow, iter};

use serde::{Deserialize, Serialize};

use currency::{AnyVisitor, FilterMapT, Matcher, MaybeAnyVisitResult, MemberOf};

use crate::{lease::Group as LeaseGroup, lpn::Group as LpnGroup, native::Group as NativeGroup};

pub use self::only::Group as OnlyGroup;
#[cfg(feature = "testing")]
pub use self::testing::{
    PaymentC1, PaymentC2, PaymentC3, PaymentC4, PaymentC5, PaymentC6, PaymentC7, PaymentC8,
    PaymentC9,
};

pub(crate) mod only;
#[cfg(feature = "testing")]
mod testing;

#[derive(Clone, Copy, Debug, Ord, PartialEq, PartialOrd, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub enum Group {}

impl currency::Group for Group {
    const DESCR: &'static str = "payment";

    type TopG = Self;

    fn maybe_visit<M, V>(matcher: &M, visitor: V) -> MaybeAnyVisitResult<Self, V>
    where
        M: Matcher,
        V: AnyVisitor<Self>,
    {
        LeaseGroup::maybe_visit_member(matcher, visitor)
            .or_else(|visitor| LpnGroup::maybe_visit_member(matcher, visitor))
            .or_else(|visitor| NativeGroup::maybe_visit_member(matcher, visitor))
            .or_else(|visitor| OnlyGroup::maybe_visit_member(matcher, visitor))
    }

    fn maybe_visit_member<M, V>(_: &M, _: V) -> MaybeAnyVisitResult<Self::TopG, V>
    where
        M: Matcher,
        V: AnyVisitor<Self::TopG>,
    {
        unreachable!()
    }

    fn filter_map<FilterMap, FilterMapRef>(
        _f: FilterMapRef,
    ) -> impl Iterator<Item = FilterMap::Outcome>
    where
        FilterMap: FilterMapT<Self>,
        FilterMapRef: Borrow<FilterMap> + Clone,
    {
        iter::empty()
    }

    fn find_map<FindMap>(_v: FindMap) -> Result<FindMap::Outcome, FindMap>
    where
        FindMap: currency::FindMapT<Self>,
    {
        todo!()
    }
}

impl MemberOf<Self> for Group {}
