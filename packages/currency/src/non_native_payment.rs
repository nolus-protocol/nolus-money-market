use serde::{Deserialize, Serialize};

use finance::currency::{AnyVisitor, Group, MaybeAnyVisitResult, Symbol, SymbolStatic};
use sdk::schemars::{self, JsonSchema};

use crate::{lease::LeaseGroup, lpn::Lpns};

#[derive(PartialEq, Eq, Clone, Copy, Debug, Serialize, Deserialize, JsonSchema)]
pub struct NonNativePaymentGroup {}

impl Group for NonNativePaymentGroup {
    const DESCR: SymbolStatic = "non_native_payment";

    fn maybe_visit_on_ticker<V>(ticker: Symbol<'_>, visitor: V) -> MaybeAnyVisitResult<V>
    where
        V: AnyVisitor,
    {
        LeaseGroup::maybe_visit_on_ticker(ticker, visitor)
            .or_else(|v| Lpns::maybe_visit_on_ticker(ticker, v))
    }

    fn maybe_visit_on_bank_symbol<V>(bank_symbol: Symbol<'_>, visitor: V) -> MaybeAnyVisitResult<V>
    where
        Self: Sized,
        V: AnyVisitor,
    {
        LeaseGroup::maybe_visit_on_bank_symbol(bank_symbol, visitor)
            .or_else(|v| Lpns::maybe_visit_on_bank_symbol(bank_symbol, v))
    }
}

#[cfg(test)]
mod test {
    use finance::currency::Currency;

    use crate::{
        lease::{Atom, Osmo, StAtom, StOsmo, Wbtc, Weth},
        lpn::Usdc,
        native::Nls,
        test::{
            maybe_visit_on_bank_symbol_err, maybe_visit_on_bank_symbol_impl,
            maybe_visit_on_ticker_err, maybe_visit_on_ticker_impl,
        },
    };

    use super::NonNativePaymentGroup as TheGroup;

    #[test]
    fn maybe_visit_on_ticker() {
        maybe_visit_on_ticker_impl::<Atom, TheGroup>();
        maybe_visit_on_ticker_impl::<StAtom, TheGroup>();
        maybe_visit_on_ticker_impl::<Osmo, TheGroup>();
        maybe_visit_on_ticker_impl::<StOsmo, TheGroup>();
        maybe_visit_on_ticker_impl::<Weth, TheGroup>();
        maybe_visit_on_ticker_impl::<Wbtc, TheGroup>();
        maybe_visit_on_ticker_impl::<Usdc, TheGroup>();
        maybe_visit_on_ticker_err::<Nls, TheGroup>(Nls::BANK_SYMBOL);
        maybe_visit_on_ticker_err::<Nls, TheGroup>(Nls::TICKER);
        maybe_visit_on_ticker_err::<Atom, TheGroup>(Atom::BANK_SYMBOL);
        maybe_visit_on_ticker_err::<Usdc, TheGroup>(Nls::BANK_SYMBOL);
        maybe_visit_on_ticker_err::<Usdc, TheGroup>(Usdc::BANK_SYMBOL);
        maybe_visit_on_ticker_err::<Osmo, TheGroup>(Atom::BANK_SYMBOL);
        maybe_visit_on_ticker_err::<Osmo, TheGroup>(Osmo::BANK_SYMBOL);
    }

    #[test]
    fn maybe_visit_on_bank_symbol() {
        maybe_visit_on_bank_symbol_impl::<Atom, TheGroup>();
        maybe_visit_on_bank_symbol_impl::<StAtom, TheGroup>();
        maybe_visit_on_bank_symbol_impl::<Osmo, TheGroup>();
        maybe_visit_on_bank_symbol_impl::<StOsmo, TheGroup>();
        maybe_visit_on_bank_symbol_impl::<Weth, TheGroup>();
        maybe_visit_on_bank_symbol_impl::<Wbtc, TheGroup>();
        maybe_visit_on_bank_symbol_impl::<Usdc, TheGroup>();
        maybe_visit_on_bank_symbol_err::<Nls, TheGroup>(Nls::TICKER);
        maybe_visit_on_bank_symbol_err::<Nls, TheGroup>(Nls::BANK_SYMBOL);
        maybe_visit_on_bank_symbol_err::<Atom, TheGroup>(Atom::TICKER);
        maybe_visit_on_bank_symbol_err::<Usdc, TheGroup>(Nls::TICKER);
        maybe_visit_on_bank_symbol_err::<Usdc, TheGroup>(Usdc::TICKER);
    }
}
