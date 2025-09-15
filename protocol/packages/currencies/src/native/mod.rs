use std::{borrow::Borrow, iter};

use serde::{Deserialize, Serialize};

use currency::{FilterMapT, FindMapT, MemberOf};

use crate::payment::Group as PaymentGroup;

pub use self::impl_mod::definitions::Nls;

#[cfg(not(feature = "testing"))]
mod impl_mod {
    include!(concat!(env!("OUT_DIR"), "/native.rs"));
}

#[cfg(feature = "testing")]
#[path = "testing.rs"]
mod impl_mod;

#[derive(Clone, Copy, Debug, Ord, PartialEq, PartialOrd, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub enum Group {}

impl currency::Group for Group {
    const DESCR: &'static str = "native";

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
        FindMap: FindMapT<TargetG = Self>,
    {
        todo!()
    }
}

impl MemberOf<Self> for Group {}

impl MemberOf<PaymentGroup> for Group {}

#[cfg(test)]
mod test {
    use currency::CurrencyDef as _;

    use crate::{
        lpn::Lpn,
        test_impl::{
            maybe_visit_on_bank_symbol_err, maybe_visit_on_bank_symbol_impl,
            maybe_visit_on_ticker_err, maybe_visit_on_ticker_impl,
        },
    };

    use super::{Group as NativeGroup, Nls};

    #[test]
    fn maybe_visit_on_ticker() {
        maybe_visit_on_ticker_impl::<Nls, NativeGroup>();
        maybe_visit_on_ticker_err::<Nls, NativeGroup>(Nls::bank());
        maybe_visit_on_ticker_err::<Nls, NativeGroup>(Lpn::ticker());
    }

    #[test]
    fn maybe_visit_on_bank_symbol() {
        maybe_visit_on_bank_symbol_impl::<Nls, NativeGroup>();
        maybe_visit_on_bank_symbol_err::<Nls, NativeGroup>(Nls::ticker());
        maybe_visit_on_bank_symbol_err::<Nls, NativeGroup>(Lpn::bank());
    }
}
