use std::borrow::Borrow;

use serde::{Deserialize, Serialize};

use currency::{GroupFilterMap, GroupFindMap, MemberOf};

use super::Group as PaymentGroup;

#[cfg(not(feature = "testing"))]
#[allow(unused_imports)]
pub(crate) use self::impl_mod::definitions::*;

mod impl_mod {
    #[cfg(feature = "testing")]
    pub(super) type Members = ();

    #[cfg(not(feature = "testing"))]
    include!(concat!(env!("OUT_DIR"), "/payment_only.rs"));
}

#[derive(Clone, Copy, Debug, Ord, PartialEq, PartialOrd, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub enum Group {}

impl currency::Group for Group {
    const DESCR: &'static str = "payment only";

    type TopG = PaymentGroup;

    type Members = self::impl_mod::Members;

    fn filter_map<FilterMap, FilterMapRef>(
        filter_map: FilterMapRef,
    ) -> impl Iterator<Item = FilterMap::Outcome>
    where
        FilterMap: GroupFilterMap<VisitedG = Self>,
        FilterMapRef: Borrow<FilterMap> + Clone,
    {
        currency::non_recursive_group_filter_map(filter_map)
    }

    fn find_map<FindMap>(find_map: FindMap) -> Result<FindMap::Outcome, FindMap>
    where
        FindMap: GroupFindMap<TargetG = Self>,
    {
        currency::non_recursive_group_find_map(find_map)
    }
}

impl MemberOf<Self> for Group {}

impl MemberOf<PaymentGroup> for Group {}
