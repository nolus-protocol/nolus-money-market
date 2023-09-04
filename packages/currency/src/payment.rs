use serde::{Deserialize, Serialize};

use sdk::schemars::{self, JsonSchema};

use crate::{
    currency::{AnyVisitor, Group, SymbolStatic},
    lease::LeaseGroup,
    lpn::Lpns,
    native::Native,
    MatcherExt,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub enum PaymentGroup {
    Lease(LeaseGroup),
    Lpns(Lpns),
    Native(Native),
}

impl Group for PaymentGroup {
    const DESCR: SymbolStatic = "payment";

    fn get_from<M: MatcherExt>(matcher: M, field_value: &M::FieldType) -> Option<Self> {
        LeaseGroup::get_from(matcher, field_value)
            .map(From::from)
            .or_else(|| Lpns::get_from(matcher, field_value).map(From::from))
            .or_else(|| Native::get_from(matcher, field_value).map(From::from))
    }

    fn visit<V: AnyVisitor>(&self, visitor: V) -> crate::AnyVisitorResult<V> {
        match self {
            PaymentGroup::Lease(lease_group) => lease_group.visit(visitor),
            PaymentGroup::Lpns(lpns) => lpns.visit(visitor),
            PaymentGroup::Native(native) => native.visit(visitor),
        }
    }
}

impl From<LeaseGroup> for PaymentGroup {
    fn from(v: LeaseGroup) -> Self {
        Self::Lease(v)
    }
}

impl From<Lpns> for PaymentGroup {
    fn from(v: Lpns) -> Self {
        Self::Lpns(v)
    }
}

impl From<Native> for PaymentGroup {
    fn from(v: Native) -> Self {
        Self::Native(v)
    }
}

#[cfg(test)]
mod test {
    use crate::{
        lease::{Atom, Osmo, StAtom, StOsmo, Wbtc, Weth},
        lpn::Usdc,
        native::Nls,
        test::group::{
            maybe_visit_on_bank_symbol_err, maybe_visit_on_bank_symbol_impl,
            maybe_visit_on_ticker_err, maybe_visit_on_ticker_impl,
        },
        Currency,
    };

    use super::PaymentGroup;

    #[test]
    fn maybe_visit_on_ticker() {
        maybe_visit_on_ticker_impl::<Atom, PaymentGroup>();
        maybe_visit_on_ticker_impl::<StAtom, PaymentGroup>();
        maybe_visit_on_ticker_impl::<Osmo, PaymentGroup>();
        maybe_visit_on_ticker_impl::<StOsmo, PaymentGroup>();
        maybe_visit_on_ticker_impl::<Weth, PaymentGroup>();
        maybe_visit_on_ticker_impl::<Wbtc, PaymentGroup>();
        maybe_visit_on_ticker_impl::<Usdc, PaymentGroup>();
        maybe_visit_on_ticker_impl::<Nls, PaymentGroup>();
        maybe_visit_on_ticker_err::<Nls, PaymentGroup>(Nls::BANK_SYMBOL);
        maybe_visit_on_ticker_err::<Atom, PaymentGroup>(Atom::BANK_SYMBOL);
        maybe_visit_on_ticker_err::<Usdc, PaymentGroup>(Nls::BANK_SYMBOL);
        maybe_visit_on_ticker_err::<Usdc, PaymentGroup>(Usdc::BANK_SYMBOL);
        maybe_visit_on_ticker_err::<Osmo, PaymentGroup>(Atom::BANK_SYMBOL);
        maybe_visit_on_ticker_err::<Osmo, PaymentGroup>(Osmo::BANK_SYMBOL);
    }

    #[test]
    fn maybe_visit_on_bank_symbol() {
        maybe_visit_on_bank_symbol_impl::<Atom, PaymentGroup>();
        maybe_visit_on_bank_symbol_impl::<StAtom, PaymentGroup>();
        maybe_visit_on_bank_symbol_impl::<Osmo, PaymentGroup>();
        maybe_visit_on_bank_symbol_impl::<StOsmo, PaymentGroup>();
        maybe_visit_on_bank_symbol_impl::<Weth, PaymentGroup>();
        maybe_visit_on_bank_symbol_impl::<Wbtc, PaymentGroup>();
        maybe_visit_on_bank_symbol_impl::<Usdc, PaymentGroup>();
        maybe_visit_on_bank_symbol_impl::<Nls, PaymentGroup>();
        maybe_visit_on_bank_symbol_err::<Nls, PaymentGroup>(Nls::TICKER);
        maybe_visit_on_bank_symbol_err::<Atom, PaymentGroup>(Atom::TICKER);
        maybe_visit_on_bank_symbol_err::<Usdc, PaymentGroup>(Nls::TICKER);
        maybe_visit_on_bank_symbol_err::<Usdc, PaymentGroup>(Usdc::TICKER);
    }
}
