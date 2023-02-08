use finance::currency::{self, AnyVisitor, Group, MaybeAnyVisitResult, Symbol, SymbolStatic};
use sdk::schemars::{self, JsonSchema};

use crate::{define_currency, define_symbol, SingleVisitorAdapter};

define_symbol! {
    USDC {
        {
            /// full ibc route: transfer/channel-0/transfer/channel-208/uusdc
            bank: "ibc/7FBDBEEEBA9C50C4BCDF7BF438EAB99E64360833D240B32655C96E319559E911",
            /// full ibc route: transfer/channel-208/uusdc
            dex: "ibc/D189335C6E4A68B513C10AB227BF1C1D38C746766278BA3EEB4FB14124F1D858",
        },
        alt: {
            /// full ibc route: transfer/channel-0/transfer/channel-312/uausdc
            // bank: "ibc/3DF30ED1342A40EA6E4941CBBE1FD4C7591F8DBA662D8162560E6A1429B018D4",
            /// full ibc route: transfer/channel-0/transfer/channel-261/udws
            bank: "ibc/9B44527BABE15E11BDB470A4B16E32A6F79D1DDE383B0687C2320B2C8614C309",
            /// full ibc route: transfer/channel-312/uausdc
            // dex: "ibc/75C8E3091D507A5A111C652F9C76C2E53059E24759A98B523723E02FA33EEF51",
            /// full ibc route: transfer/channel-261/udws
            /// borrowed from osmosis.pool_id = 672 as a currency with enough liquidity
            dex: "ibc/902BDADA0D46931BF5DEBE0648CC1FE137AA4B7346475DD0490D503C937A12BD",
        },
    }
}
define_currency!(Usdc, USDC);

#[derive(Clone, PartialEq, Eq, JsonSchema)]
#[cfg_attr(any(test, feature = "testing"), derive(Debug))]
pub struct Lpns {}
impl Group for Lpns {
    const DESCR: SymbolStatic = "lpns";

    fn maybe_visit_on_ticker<V>(ticker: Symbol<'_>, visitor: V) -> MaybeAnyVisitResult<V>
    where
        V: AnyVisitor,
    {
        let v: SingleVisitorAdapter<_> = visitor.into();
        currency::maybe_visit_on_ticker::<Usdc, _>(ticker, v).map_err(|v| v.0)
    }

    fn maybe_visit_on_bank_symbol<V>(bank_symbol: Symbol<'_>, visitor: V) -> MaybeAnyVisitResult<V>
    where
        Self: Sized,
        V: AnyVisitor,
    {
        let v: SingleVisitorAdapter<_> = visitor.into();
        currency::maybe_visit_on_bank_symbol::<Usdc, _>(bank_symbol, v).map_err(|v| v.0)
    }
}

#[cfg(test)]
mod test {
    use finance::currency::Currency;

    use crate::{
        lease::Osmo,
        native::Nls,
        test::{
            maybe_visit_on_bank_symbol_err, maybe_visit_on_bank_symbol_impl,
            maybe_visit_on_ticker_err, maybe_visit_on_ticker_impl,
        },
    };

    use super::{Lpns, Usdc};

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
