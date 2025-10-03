use std::borrow::Borrow;

use serde::{Deserialize, Serialize};

use currency::{CurrenciesMapping, GroupFilterMap, GroupFindMap, MemberOf, group_find_map};

use super::Group as PaymentGroup;

use self::impl_mod::GroupMember;
#[cfg(not(feature = "testing"))]
#[allow(unused_imports)]
pub(crate) use self::impl_mod::definitions::*;

#[cfg(not(feature = "testing"))]
mod impl_mod {
    include!(concat!(env!("OUT_DIR"), "/payment_only.rs"));
}

#[cfg(feature = "testing")]
#[path = "testing.rs"]
mod impl_mod;

#[derive(Clone, Copy, Debug, Ord, PartialEq, PartialOrd, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub enum Group {}

impl currency::Group for Group {
    const DESCR: &'static str = "payment only";

    type TopG = PaymentGroup;

    fn filter_map<FilterMap, FilterMapRef>(
        filter_map: FilterMapRef,
    ) -> impl Iterator<Item = FilterMap::Outcome>
    where
        FilterMap: GroupFilterMap<VisitedG = Self>,
        FilterMapRef: Borrow<FilterMap> + Clone,
    {
        CurrenciesMapping::<_, GroupMember, _, _>::with_filter(filter_map)
    }

    fn find_map<FindMap>(find_map: FindMap) -> Result<FindMap::Outcome, FindMap>
    where
        FindMap: GroupFindMap<TargetG = Self>,
    {
        group_find_map::<_, GroupMember, _>(find_map)
    }
}

impl MemberOf<Self> for Group {}

impl MemberOf<PaymentGroup> for Group {}
