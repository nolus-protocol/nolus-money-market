use std::borrow::Borrow;

use serde::{Deserialize, Serialize};

use currency::{GroupFilterMapT, GroupFindMapT, MemberOf, SubFilterAdapter, SubGroupFindAdapter};

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

    fn filter_map<FilterMap, FilterMapRef>(
        filter_map: FilterMapRef,
    ) -> impl Iterator<Item = FilterMap::Outcome>
    where
        FilterMap: GroupFilterMapT<VisitedG = Self>,
        FilterMapRef: Borrow<FilterMap> + Clone,
    {
        LpnGroup::filter_map(SubFilterAdapter::new(filter_map.clone()))
            .chain(NativeGroup::filter_map(SubFilterAdapter::new(
                filter_map.clone(),
            )))
            .chain(LeaseGroup::filter_map(SubFilterAdapter::new(
                filter_map.clone(),
            )))
            .chain(OnlyGroup::filter_map(SubFilterAdapter::new(filter_map)))
    }

    fn find_map<FindMap>(find_map: FindMap) -> Result<FindMap::Outcome, FindMap>
    where
        FindMap: GroupFindMapT<TargetG = Self>,
    {
        LpnGroup::find_map(SubGroupFindAdapter::new(find_map))
            .or_else(|find_map| {
                NativeGroup::find_map(SubGroupFindAdapter::new(find_map.release_super_map()))
            })
            .or_else(|find_map| {
                LeaseGroup::find_map(SubGroupFindAdapter::new(find_map.release_super_map()))
            })
            .or_else(|find_map| {
                OnlyGroup::find_map(SubGroupFindAdapter::new(find_map.release_super_map()))
            })
            .map_err(SubGroupFindAdapter::release_super_map)
    }
}

impl MemberOf<Self> for Group {}
