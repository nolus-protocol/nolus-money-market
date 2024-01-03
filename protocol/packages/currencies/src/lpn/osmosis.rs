use currency::{AnyVisitor, Matcher, MaybeAnyVisitResult, SymbolSlice};
use sdk::schemars;

use crate::{
    define_currency, define_symbol,
    ibc::macros::{bank_symbol, dex_symbol},
};

define_symbol! {
    USDC {
        ["net_dev", "net_test"]: {
            bank: bank_symbol!([3], "uausdc"),
            dex: dex_symbol!([3], "uausdc"),
        },
        ["net_main"]: {
            bank: bank_symbol!([208], "uusdc"),
            dex: dex_symbol!([208], "uusdc"),
        },
    }
}
define_currency!(Usdc, USDC);

pub(super) fn maybe_visit<M, V>(
    matcher: &M,
    symbol: &SymbolSlice,
    visitor: V,
) -> MaybeAnyVisitResult<V>
where
    M: Matcher + ?Sized,
    V: AnyVisitor,
{
    use currency::maybe_visit_any as maybe_visit;
    maybe_visit::<_, Usdc, _>(matcher, symbol, visitor)
}

#[cfg(test)]
mod test {
    use currency::Currency;

    use crate::{
        lease::osmosis::Osmo,
        lpn::{osmosis::Usdc, Lpns},
        native::osmosis::Nls,
        test_impl::{
            maybe_visit_on_bank_symbol_err, maybe_visit_on_bank_symbol_impl,
            maybe_visit_on_ticker_err, maybe_visit_on_ticker_impl,
        },
    };

    #[test]
    fn maybe_visit_on_ticker() {
        maybe_visit_on_ticker_impl::<Usdc, Lpns>();
        maybe_visit_on_ticker_err::<Usdc, Lpns>(Usdc::BANK_SYMBOL);
        maybe_visit_on_ticker_err::<Usdc, Lpns>(Nls::TICKER);
        maybe_visit_on_ticker_err::<Usdc, Lpns>(Osmo::TICKER);
    }

    #[test]
    fn maybe_visit_on_bank_symbol() {
        maybe_visit_on_bank_symbol_impl::<Usdc, Lpns>();
        maybe_visit_on_bank_symbol_err::<Usdc, Lpns>(Usdc::TICKER);
        maybe_visit_on_bank_symbol_err::<Usdc, Lpns>(Nls::BANK_SYMBOL);
        maybe_visit_on_bank_symbol_err::<Usdc, Lpns>(Osmo::BANK_SYMBOL);
    }
}
