use std::borrow::Borrow;

use serde::{Deserialize, Serialize};

use currency::{
    CurrenciesMapping, GroupFilterMapT, GroupFindMapT, MemberOf, PairsFindMapT, PairsGroup,
};

use crate::payment::Group as PaymentGroup;

use self::impl_mod::GroupMember;
pub use self::impl_mod::definitions::Lpn;

#[cfg(not(feature = "testing"))]
mod impl_mod {
    include!(concat!(env!("OUT_DIR"), "/lpn.rs"));
}

#[cfg(feature = "testing")]
#[path = "testing.rs"]
mod impl_mod;

#[derive(Clone, Copy, Debug, Ord, PartialEq, PartialOrd, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub enum Group {}

impl currency::Group for Group {
    const DESCR: &'static str = "lpns";

    type TopG = PaymentGroup;

    fn filter_map<FilterMap, FilterMapRef>(
        filter_map: FilterMapRef,
    ) -> impl Iterator<Item = FilterMap::Outcome>
    where
        FilterMap: GroupFilterMapT<VisitedG = Self>,
        FilterMapRef: Borrow<FilterMap> + Clone,
    {
        CurrenciesMapping::<_, GroupMember, _, _>::with_filter(filter_map)
    }

    fn find_map<FindMap>(find_map: FindMap) -> Result<FindMap::Outcome, FindMap>
    where
        FindMap: GroupFindMapT<TargetG = Self>,
    {
        currency::group_find_map::<_, GroupMember, _>(find_map)
    }
}

impl MemberOf<Self> for Group {}

impl MemberOf<PaymentGroup> for Group {}

impl PairsGroup for Lpn {
    type CommonGroup = PaymentGroup;

    fn find_map<FindMap>(find_map: FindMap) -> Result<FindMap::Outcome, FindMap>
    where
        FindMap: PairsFindMapT<Pivot = Self>,
    {
        Err(find_map)
    }
}

#[cfg(test)]
mod test {
    use currency::CurrencyDef as _;

    use crate::{
        native::Nls,
        test_impl::{
            maybe_visit_on_bank_symbol_err, maybe_visit_on_bank_symbol_impl,
            maybe_visit_on_ticker_err, maybe_visit_on_ticker_impl,
        },
    };

    use super::{Group as Lpns, Lpn};

    #[test]
    fn maybe_visit_on_ticker() {
        maybe_visit_on_ticker_impl::<Lpn, Lpns>();
        maybe_visit_on_ticker_err::<Lpn, Lpns>(Lpn::bank());
        maybe_visit_on_ticker_err::<Lpn, Lpns>(Nls::ticker());
    }

    #[test]
    fn maybe_visit_on_bank_symbol() {
        maybe_visit_on_bank_symbol_impl::<Lpn, Lpns>();
        maybe_visit_on_bank_symbol_err::<Lpn, Lpns>(Lpn::ticker());
        maybe_visit_on_bank_symbol_err::<Lpn, Lpns>(Nls::bank());
    }
}
