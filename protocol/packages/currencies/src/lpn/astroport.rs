use currency::{AnyVisitor, Matcher, MaybeAnyVisitResult, SymbolSlice};
use sdk::schemars;

use crate::{
    define_currency, define_symbol,
    ibc::macros::{bank_symbol, dex_symbol},
};

define_symbol! {
    USDC_AXELAR {
        ["net_dev", "net_test"]: {
            bank: bank_symbol!([8], "uausdc"),
            dex: dex_symbol!([8], "uausdc"),
        },
        ["net_main"]: {
            bank: bank_symbol!([2], "uusdc"),
            dex: dex_symbol!([2], "uusdc"),
        },
    }
}
define_currency!(UsdcAxelar, USDC_AXELAR);

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
    maybe_visit::<_, UsdcAxelar, _>(matcher, symbol, visitor)
}

#[cfg(test)]
mod test {
    use currency::Currency;

    use crate::{
        lease::astroport::Ntrn,
        lpn::Lpns,
        native::Nls,
        test_impl::{
            maybe_visit_on_bank_symbol_err, maybe_visit_on_bank_symbol_impl,
            maybe_visit_on_ticker_err, maybe_visit_on_ticker_impl,
        },
    };

    use super::UsdcAxelar;

    #[test]
    fn maybe_visit_on_ticker() {
        maybe_visit_on_ticker_impl::<UsdcAxelar, Lpns>();
        maybe_visit_on_ticker_err::<UsdcAxelar, Lpns>(UsdcAxelar::BANK_SYMBOL);
        maybe_visit_on_ticker_err::<UsdcAxelar, Lpns>(Nls::TICKER);
        maybe_visit_on_ticker_err::<UsdcAxelar, Lpns>(Ntrn::TICKER);
    }

    #[test]
    fn maybe_visit_on_bank_symbol() {
        maybe_visit_on_bank_symbol_impl::<UsdcAxelar, Lpns>();
        maybe_visit_on_bank_symbol_err::<UsdcAxelar, Lpns>(UsdcAxelar::TICKER);
        maybe_visit_on_bank_symbol_err::<UsdcAxelar, Lpns>(Nls::BANK_SYMBOL);
        maybe_visit_on_bank_symbol_err::<UsdcAxelar, Lpns>(Ntrn::BANK_SYMBOL);
    }
}
