use std::{borrow::Borrow, iter};

use serde::{Deserialize, Serialize};

use currency::{FilterMapT, MemberOf};

use super::Group as PaymentGroup;

#[cfg(not(feature = "testing"))]
#[allow(unused_imports)]
pub(crate) use self::impl_mod::definitions::*;

#[cfg(not(feature = "testing"))]
mod impl_mod {
    include!(concat!(env!("OUT_DIR"), "/payment_only.rs"));
}

#[derive(Clone, Copy, Debug, Ord, PartialEq, PartialOrd, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub enum Group {}

impl currency::Group for Group {
    const DESCR: &'static str = "payment only";

    type TopG = PaymentGroup;

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

impl MemberOf<PaymentGroup> for Group {}
