use finance::currency::{self, AnyVisitor, Group, MaybeAnyVisitResult, Symbol, SymbolStatic};

use crate::{lease::LeaseGroup, lpn::Lpns, native::Nls, SingleVisitorAdapter};

pub struct PaymentGroup {}

impl Group for PaymentGroup {
    const DESCR: SymbolStatic = "payment";

    fn maybe_visit_on_ticker<V>(ticker: Symbol, visitor: V) -> MaybeAnyVisitResult<V>
    where
        V: AnyVisitor,
    {
        LeaseGroup::maybe_visit_on_ticker(ticker, visitor)
            .or_else(|v| Lpns::maybe_visit_on_ticker(ticker, v))
            .or_else(|v| {
                currency::maybe_visit_on_ticker::<Nls, _>(ticker, SingleVisitorAdapter::from(v))
            })
            .map_err(|v| v.0)
    }

    fn maybe_visit_on_bank_symbol<V>(bank_symbol: Symbol, visitor: V) -> MaybeAnyVisitResult<V>
    where
        Self: Sized,
        V: AnyVisitor,
    {
        LeaseGroup::maybe_visit_on_bank_symbol(bank_symbol, visitor)
            .or_else(|v| Lpns::maybe_visit_on_bank_symbol(bank_symbol, v))
            .or_else(|v| {
                currency::maybe_visit_on_bank_symbol::<Nls, _>(
                    bank_symbol,
                    SingleVisitorAdapter::from(v),
                )
            })
            .map_err(|v| v.0)
    }
}

#[cfg(test)]
mod test {
    use finance::currency::Currency;

    use crate::{
        lease::{Atom, Osmo, Wbtc, Weth},
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
        maybe_visit_on_ticker_impl::<Osmo, PaymentGroup>();
        maybe_visit_on_ticker_impl::<Weth, PaymentGroup>();
        maybe_visit_on_ticker_impl::<Wbtc, PaymentGroup>();
        maybe_visit_on_ticker_impl::<Usdc, PaymentGroup>();
        maybe_visit_on_ticker_impl::<Nls, PaymentGroup>();
        maybe_visit_on_ticker_err::<Atom, PaymentGroup>(Atom::BANK_SYMBOL);
        maybe_visit_on_ticker_err::<Usdc, PaymentGroup>(Nls::BANK_SYMBOL);
        maybe_visit_on_ticker_err::<Usdc, PaymentGroup>(Usdc::BANK_SYMBOL);
    }

    #[test]
    fn maybe_visit_on_bank_symbol() {
        maybe_visit_on_bank_symbol_impl::<Atom, PaymentGroup>();
        maybe_visit_on_bank_symbol_impl::<Osmo, PaymentGroup>();
        maybe_visit_on_bank_symbol_impl::<Weth, PaymentGroup>();
        maybe_visit_on_bank_symbol_impl::<Wbtc, PaymentGroup>();
        maybe_visit_on_bank_symbol_impl::<Usdc, PaymentGroup>();
        maybe_visit_on_bank_symbol_impl::<Nls, PaymentGroup>();
        maybe_visit_on_bank_symbol_err::<Atom, PaymentGroup>(Atom::TICKER);
        maybe_visit_on_bank_symbol_err::<Usdc, PaymentGroup>(Nls::TICKER);
        maybe_visit_on_bank_symbol_err::<Usdc, PaymentGroup>(Usdc::TICKER);
    }
}
