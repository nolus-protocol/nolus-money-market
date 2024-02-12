use currency::{AnyVisitor, Matcher, MaybeAnyVisitResult, SymbolSlice};
use sdk::schemars;

use crate::{define_currency, define_symbol};

define_symbol! {
    USDC_NOBLE {
        ["net_dev"]: {
            // full ibc route: transfer/channel-0/transfer/channel-???/uusdc
            bank: "ibc/NA_USDC_NOBLE",
            // full ibc route: transfer/channel-???/uusdc
            dex: "ibc/NA_USDC_NOBLE_DEX",
        },
        ["net_test"]: {
            // full ibc route: transfer/channel-1993/transfer/channel-???/uusdc
            bank: "ibc/NA_USDC_NOBLE",
            // full ibc route: transfer/channel-???/uusdc
            dex: "ibc/NA_USDC_NOBLE_DEX",
        },
        ["net_main"]: {
            // full ibc route: transfer/channel-0/transfer/channel-750/uusdc
            bank: "ibc/F5FABF52B54E65064B57BF6DBD8E5FAD22CEE9F4B8A57ADBB20CCD0173AA72A4",
            // full ibc route: transfer/channel-750/uusdc
            dex: "ibc/498A0751C798A0D9A389AA3691123DADA57DAA4FE165D5C75894505B876BA6E4",
        },
    }
}
define_currency!(UsdcNoble, USDC_NOBLE);

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
    maybe_visit::<_, UsdcNoble, _>(matcher, symbol, visitor)
}

#[cfg(test)]
mod test {
    use currency::Currency;

    use crate::{
        lease::osmosis::Osmo,
        lpn::{osmosis::Usdc, Lpns},
        native::osmosis::Nls,
        payment::only::PaymentOnlyGroup,
        test_impl::{
            maybe_visit_on_bank_symbol_err, maybe_visit_on_bank_symbol_impl,
            maybe_visit_on_ticker_err, maybe_visit_on_ticker_impl,
        },
    };

    use super::UsdcNoble;

    #[test]
    fn maybe_visit_on_ticker() {
        maybe_visit_on_ticker_impl::<UsdcNoble, PaymentOnlyGroup>();
        maybe_visit_on_ticker_err::<UsdcNoble, PaymentOnlyGroup>(UsdcNoble::BANK_SYMBOL);
        maybe_visit_on_ticker_err::<UsdcNoble, PaymentOnlyGroup>(Usdc::TICKER);
        maybe_visit_on_ticker_err::<Usdc, Lpns>(UsdcNoble::TICKER);
    }

    #[test]
    fn maybe_visit_on_bank_symbol() {
        maybe_visit_on_bank_symbol_impl::<UsdcNoble, PaymentOnlyGroup>();
        maybe_visit_on_bank_symbol_err::<UsdcNoble, PaymentOnlyGroup>(UsdcNoble::TICKER);
        maybe_visit_on_bank_symbol_err::<UsdcNoble, PaymentOnlyGroup>(Nls::BANK_SYMBOL);
        maybe_visit_on_bank_symbol_err::<UsdcNoble, PaymentOnlyGroup>(Osmo::BANK_SYMBOL);
        maybe_visit_on_bank_symbol_err::<UsdcNoble, PaymentOnlyGroup>(Usdc::BANK_SYMBOL);
    }
}
