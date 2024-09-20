use serde::{Deserialize, Serialize};

use currency::{AnyVisitor, Matcher, MaybeAnyVisitResult, MemberOf};
use sdk::schemars::JsonSchema;

use crate::payment;

#[cfg(not(feature = "testing"))]
pub(crate) mod impl_mod {
    include!(concat!(env!("OUT_DIR"), "/lease.rs"));
}

#[cfg(feature = "testing")]
#[path = "testing.rs"]
pub(crate) mod impl_mod;

#[derive(
    Clone, Copy, Debug, Ord, PartialEq, PartialOrd, Eq, Serialize, Deserialize, JsonSchema,
)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
#[schemars(crate = "sdk::schemars")]
pub enum Group {}

impl currency::Group for Group {
    const DESCR: &'static str = "lease";

    type TopG = payment::Group;

    #[inline]
    fn maybe_visit<M, V>(matcher: &M, visitor: V) -> MaybeAnyVisitResult<Self, V>
    where
        M: Matcher,
        V: AnyVisitor<Self>,
    {
        impl_mod::maybe_visit(matcher, visitor)
    }

    #[inline]
    fn maybe_visit_member<M, V>(matcher: &M, visitor: V) -> MaybeAnyVisitResult<Self::TopG, V>
    where
        M: Matcher,
        V: AnyVisitor<Self::TopG>,
    {
        impl_mod::maybe_visit(matcher, visitor)
    }
}

impl MemberOf<Self> for Group {}

impl MemberOf<payment::Group> for Group {}

#[cfg(all(feature = "testing", test))]
mod test {
    use currency::CurrencyDef as _;

    use crate::{
        lpn::impl_mod::Lpn,
        native::impl_mod::Nls,
        test_impl::{
            maybe_visit_on_bank_symbol_err, maybe_visit_on_bank_symbol_impl,
            maybe_visit_on_ticker_err, maybe_visit_on_ticker_impl,
        },
    };

    use super::{impl_mod::LeaseC1, Group};

    #[test]
    fn maybe_visit_on_ticker() {
        maybe_visit_on_ticker_impl::<LeaseC1, Group>();
        maybe_visit_on_ticker_err::<LeaseC1, Group>(Lpn::ticker());
        maybe_visit_on_ticker_err::<LeaseC1, Group>(Nls::ticker());
    }

    #[test]
    fn maybe_visit_on_bank_symbol() {
        maybe_visit_on_bank_symbol_impl::<LeaseC1, Group>();
        maybe_visit_on_bank_symbol_err::<LeaseC1, Group>(Lpn::bank());
        maybe_visit_on_bank_symbol_err::<LeaseC1, Group>(Nls::bank());
    }
}
