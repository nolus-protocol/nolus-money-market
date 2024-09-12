use serde::{Deserialize, Serialize};

use currency::{
    AnyVisitor, Group, Matcher, MaybeAnyVisitResult, MaybePairsVisitorResult, MemberOf, PairsGroup,
    PairsVisitor,
};
use sdk::schemars::{self, JsonSchema};

pub use impl_mod::Lpn;

#[cfg(not(feature = "testing"))]
use r#impl as impl_mod;
#[cfg(feature = "testing")]
use testing as impl_mod;

use crate::PaymentGroup;

#[cfg(not(feature = "testing"))]
mod r#impl;
#[cfg(feature = "testing")]
mod testing;

impl PairsGroup for Lpn {
    type CommonGroup = PaymentGroup;

    fn maybe_visit<M, V>(_matcher: &M, visitor: V) -> MaybePairsVisitorResult<V>
    where
        M: Matcher,
        V: PairsVisitor<Pivot = Self>,
    {
        currency::visit_noone(visitor)
    }
}

#[derive(
    Clone, Copy, Debug, Ord, PartialEq, PartialOrd, Eq, JsonSchema, Serialize, Deserialize,
)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct Lpns {}

impl Group for Lpns {
    const DESCR: &'static str = "lpns";
    type TopG = PaymentGroup;

    fn maybe_visit<M, V>(matcher: &M, visitor: V) -> MaybeAnyVisitResult<Self, V>
    where
        M: Matcher,
        V: AnyVisitor<Self>,
    {
        currency::maybe_visit_member::<_, Lpn, Self, _>(matcher, visitor)
    }

    fn maybe_visit_member<M, V>(matcher: &M, visitor: V) -> MaybeAnyVisitResult<Self::TopG, V>
    where
        M: Matcher,
        V: AnyVisitor<Self::TopG>,
    {
        currency::maybe_visit_member::<_, Lpn, Self::TopG, _>(matcher, visitor)
    }
}

impl MemberOf<PaymentGroup> for Lpns {}
impl MemberOf<Self> for Lpns {}

#[cfg(test)]
mod test {
    use currency::CurrencyDef as _;

    use crate::{
        lpn::{Lpn, Lpns},
        native::Nls,
        test_impl::{
            maybe_visit_on_bank_symbol_err, maybe_visit_on_bank_symbol_impl,
            maybe_visit_on_ticker_err, maybe_visit_on_ticker_impl,
        },
    };

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
