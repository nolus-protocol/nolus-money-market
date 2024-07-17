use serde::{Deserialize, Serialize};

use currency::{AnyVisitor, Group, Matcher, MaybeAnyVisitResult, MemberOf};
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, JsonSchema, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct Lpns {}

impl Group for Lpns {
    const DESCR: &'static str = "lpns";

    fn maybe_visit<M, V>(matcher: &M, visitor: V) -> MaybeAnyVisitResult<V>
    where
        M: Matcher<Group = Self>,
        V: AnyVisitor<VisitedG = Self>,
    {
        Self::maybe_visit_member(matcher, visitor)
    }

    fn maybe_visit_member<M, V>(matcher: &M, visitor: V) -> MaybeAnyVisitResult<V>
    where
        M: Matcher,
        V: AnyVisitor,
        Self: MemberOf<V::VisitedG> + MemberOf<M::Group>,
    {
        currency::maybe_visit_any::<_, Lpn, _>(matcher, visitor)
    }
}

impl MemberOf<PaymentGroup> for Lpns {}
impl MemberOf<Self> for Lpns {}

#[cfg(test)]
mod test {
    use currency::Currency;

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
        maybe_visit_on_ticker_err::<Lpn, Lpns>(Lpn::BANK_SYMBOL);
        maybe_visit_on_ticker_err::<Lpn, Lpns>(Nls::TICKER);
    }

    #[test]
    fn maybe_visit_on_bank_symbol() {
        maybe_visit_on_bank_symbol_impl::<Lpn, Lpns>();
        maybe_visit_on_bank_symbol_err::<Lpn, Lpns>(Lpn::TICKER);
        maybe_visit_on_bank_symbol_err::<Lpn, Lpns>(Nls::BANK_SYMBOL);
    }
}
