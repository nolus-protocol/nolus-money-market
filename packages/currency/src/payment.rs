use serde::{Deserialize, Serialize};

use finance::currency::{AnyVisitor, Group, MaybeAnyVisitResult, Symbol, SymbolStatic};
use sdk::schemars::{self, JsonSchema};

use crate::{native::Native, non_native_payment::NonNativePaymentGroup};

#[derive(PartialEq, Eq, Clone, Copy, Debug, Serialize, Deserialize, JsonSchema)]
pub struct PaymentGroup {}

impl Group for PaymentGroup {
    const DESCR: SymbolStatic = "payment";

    fn maybe_visit_on_ticker<V>(ticker: Symbol<'_>, visitor: V) -> MaybeAnyVisitResult<V>
    where
        V: AnyVisitor,
    {
        NonNativePaymentGroup::maybe_visit_on_ticker(ticker, visitor)
            .or_else(|v| Native::maybe_visit_on_ticker(ticker, v))
    }

    fn maybe_visit_on_bank_symbol<V>(bank_symbol: Symbol<'_>, visitor: V) -> MaybeAnyVisitResult<V>
    where
        Self: Sized,
        V: AnyVisitor,
    {
        NonNativePaymentGroup::maybe_visit_on_bank_symbol(bank_symbol, visitor)
            .or_else(|v| Native::maybe_visit_on_bank_symbol(bank_symbol, v))
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
