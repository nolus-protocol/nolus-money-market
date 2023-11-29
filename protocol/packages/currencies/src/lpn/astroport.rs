use sdk::schemars;

use currency::{AnyVisitor, Matcher, MaybeAnyVisitResult, SymbolSlice};

use crate::{define_currency, define_symbol};

define_symbol! {
    USDC_AXELAR {
        ["dev"]: {
            /// full ibc route: transfer/channel-116/transfer/channel-8/uausdc
            bank: "ibc/B3F73CBDD3C286B8EA46FA9100A114B91731F0F4A23660FBAA47DCB7AAA968AB",
            /// full ibc route: transfer/channel-8/uausdc
            dex: "ibc/F91EA2C0A23697A1048E08C2F787E3A58AC6F706A1CD2257A504925158CFC0F3",
        },
        ["test"]: {
            /// full ibc route: transfer/channel-1990/transfer/channel-8/uausdc
            bank: "ibc/88E889952D6F30CEFCE1B1EE4089DA54939DE44B0A7F11558C230209AF228937",
            /// full ibc route: transfer/channel-8/uausdc
            dex: "ibc/F91EA2C0A23697A1048E08C2F787E3A58AC6F706A1CD2257A504925158CFC0F3",
        },
        ["main"]: {
            /// full ibc route: transfer/channel-3839/transfer/channel-?/uusdc
            bank: "ibc/NA_USDC_AXELAR",
            /// full ibc route: transfer/channel-?/uusdc
            dex: "ibc/NA_USDC_AXELAR_DEX",
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
