use std::borrow::Borrow;

use serde::{Deserialize, Serialize};

use currency::{CurrenciesMapping, GroupFilterMap, GroupFindMap, MemberOf};

use crate::payment::Group as PaymentGroup;

use self::impl_mod::GroupMember;
// TODO use cfg_match! once gets stabilized
#[cfg(not(feature = "testing"))]
#[allow(unused_imports)]
pub(crate) use self::impl_mod::definitions::*;
#[cfg(feature = "testing")]
pub use self::impl_mod::definitions::{
    LeaseC1, LeaseC2, LeaseC3, LeaseC4, LeaseC5, LeaseC6, LeaseC7,
};

#[cfg(not(feature = "testing"))]
mod impl_mod {
    include!(concat!(env!("OUT_DIR"), "/lease.rs"));
}

#[cfg(feature = "testing")]
#[path = "testing.rs"]
mod impl_mod;

#[derive(Clone, Copy, Debug, Ord, PartialEq, PartialOrd, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub enum Group {}

impl currency::Group for Group {
    const DESCR: &'static str = "lease";

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
        currency::group_find_map::<_, GroupMember, _>(find_map)
    }
}

impl MemberOf<Self> for Group {}

impl MemberOf<PaymentGroup> for Group {}

#[cfg(all(feature = "testing", test))]
mod test {
    use currency::CurrencyDef as _;

    use crate::{lpn::Lpn, native::Nls, test_impl};

    use super::{Group, LeaseC1};

    #[test]
    fn maybe_visit_on_ticker() {
        test_impl::maybe_visit_on_ticker_impl::<LeaseC1, Group>();
        test_impl::maybe_visit_on_ticker_err::<LeaseC1, Group>(Lpn::ticker());
        test_impl::maybe_visit_on_ticker_err::<LeaseC1, Group>(Nls::ticker());
    }

    #[test]
    fn maybe_visit_on_bank_symbol() {
        test_impl::maybe_visit_on_bank_symbol_impl::<LeaseC1, Group>();
        test_impl::maybe_visit_on_bank_symbol_err::<LeaseC1, Group>(Lpn::bank());
        test_impl::maybe_visit_on_bank_symbol_err::<LeaseC1, Group>(Nls::bank());
    }
}
