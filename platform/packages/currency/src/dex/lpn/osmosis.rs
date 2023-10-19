use sdk::schemars;

use crate::{
    define_currency, define_symbol, AnyVisitor, Matcher, MaybeAnyVisitResult, SymbolSlice,
};

define_symbol! {
    USDC {
        ["dev", "test"]: {
            /// full ibc route: transfer/channel-0/transfer/channel-3/uausdc
            bank: "ibc/5DE4FCAF68AE40F81F738C857C0D95F7C1BC47B00FA1026E85C1DD92524D4A11",
            /// full ibc route: transfer/channel-3/uausdc
            dex: "ibc/6F34E1BD664C36CE49ACC28E60D62559A5F96C4F9A6CCE4FC5A67B2852E24CFE",
        },
        ["main"]: {
            /// full ibc route: transfer/channel-0/transfer/channel-208/uusdc
            bank: "ibc/7FBDBEEEBA9C50C4BCDF7BF438EAB99E64360833D240B32655C96E319559E911",
            /// full ibc route: transfer/channel-208/uusdc
            dex: "ibc/D189335C6E4A68B513C10AB227BF1C1D38C746766278BA3EEB4FB14124F1D858",
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
    use crate::maybe_visit_any as maybe_visit;
    maybe_visit::<_, Usdc, _>(matcher, symbol, visitor)
}

#[cfg(test)]
mod test {
    use crate::{
        dex::{
            lease::osmosis::Osmo,
            lpn::{osmosis::Usdc, Lpns},
            native::osmosis::Nls,
            test_impl::{
                maybe_visit_on_bank_symbol_err, maybe_visit_on_bank_symbol_impl,
                maybe_visit_on_ticker_err, maybe_visit_on_ticker_impl,
            },
        },
        Currency,
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
