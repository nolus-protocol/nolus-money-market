use currency::{AnyVisitor, Matcher, MaybeAnyVisitResult, SymbolSlice};
use sdk::schemars;

use crate::{
    define_currency, define_symbol,
    ibc::macros::{bank_symbol, dex_native_symbol, dex_symbol},
};

#[cfg(feature = "testing")]
pub use self::testing_currencies::*;

// Resources:
// 1. Symbol hashes are computed using the SHA256 Hash Generator https://coding.tools/sha256
// 2. Currencies that come from Axelar are documented at https://docs.axelar.dev/resources
// 3. IBC routes from https://github.com/Nolus-Protocol/Wiki/blob/main/testnet-rila/currencies.json

define_symbol! {
    ATOM {
        ["net_dev", "net_test", "net_main"]: {
            bank: bank_symbol!([1], "uatom"),
            dex: dex_symbol!([1], "uatom"),
        },
    }
}
define_currency!(Atom, ATOM);

define_symbol! {
    ST_ATOM {
        ["net_dev", "net_test"]: {
            bank: crate::symbols_macro::BankSymbol("ibc/NA_ST_ATOM"),
            dex: crate::symbols_macro::DexSymbol("ibc/NA_ST_ATOM_DEX"),
        },
        ["net_main"]: {
            bank: bank_symbol!([8], "stuatom"),
            dex: dex_symbol!([8], "stuatom"),
        },
    }
}
define_currency!(StAtom, ST_ATOM);

define_symbol! {
    NTRN {
        ["net_dev", "net_test", "net_main"]: {
            bank: bank_symbol!([], "untrn"),
            dex: dex_native_symbol!("untrn"),
        },
    }
}
define_currency!(Ntrn, NTRN);

define_symbol! {
    DYDX {
        ["net_dev", "net_test"]: {
            bank: crate::symbols_macro::BankSymbol("ibc/NA_DYDX"),
            dex: crate::symbols_macro::DexSymbol("ibc/NA_DYDX_DEX"),
        },
        ["net_main"]: {
            bank: bank_symbol!([48], "adydx"),
            dex: dex_symbol!([48], "adydx"),
        },
    }
}
define_currency!(Dydx, DYDX);

#[cfg(feature = "testing")]
mod testing_currencies {
    use sdk::schemars;

    use crate::{
        define_currency, define_symbol,
        symbols_macro::{BankSymbol, DexSymbol},
    };

    define_symbol! {
        TEST_C1 {
            ["net_dev", "net_test", "net_main"]: {
                bank: BankSymbol("ibc/test_currency_1"),
                dex: DexSymbol("ibc/test_currency_1_dex"),
            },
        }
    }
    define_currency!(TestC1, TEST_C1);
}

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
    let result = maybe_visit::<_, Atom, _>(matcher, symbol, visitor)
        .or_else(|visitor| maybe_visit::<_, StAtom, _>(matcher, symbol, visitor))
        .or_else(|visitor| maybe_visit::<_, Ntrn, _>(matcher, symbol, visitor))
        .or_else(|visitor| maybe_visit::<_, Dydx, _>(matcher, symbol, visitor));

    #[cfg(not(feature = "testing"))]
    {
        result
    }
    #[cfg(feature = "testing")]
    result.or_else(|visitor| maybe_visit_test_currencies(matcher, symbol, visitor))
}

#[cfg(feature = "testing")]
fn maybe_visit_test_currencies<M, V>(
    matcher: &M,
    symbol: &SymbolSlice,
    visitor: V,
) -> MaybeAnyVisitResult<V>
where
    M: Matcher + ?Sized,
    V: AnyVisitor,
{
    use currency::maybe_visit_any as maybe_visit;
    maybe_visit::<_, TestC1, _>(matcher, symbol, visitor)
}

#[cfg(test)]
mod test {
    use currency::Currency;

    use crate::{
        test_impl::{
            maybe_visit_on_bank_symbol_err, maybe_visit_on_bank_symbol_impl,
            maybe_visit_on_ticker_err, maybe_visit_on_ticker_impl,
        },
        {lease::LeaseGroup, lpn::astroport::UsdcAxelar, native::Nls},
    };

    use super::{Atom, Dydx, Ntrn, StAtom};

    #[test]
    fn maybe_visit_on_ticker() {
        maybe_visit_on_ticker_impl::<Atom, LeaseGroup>();
        maybe_visit_on_ticker_impl::<StAtom, LeaseGroup>();
        maybe_visit_on_ticker_impl::<Ntrn, LeaseGroup>();
        maybe_visit_on_ticker_impl::<Dydx, LeaseGroup>();
        maybe_visit_on_ticker_err::<UsdcAxelar, LeaseGroup>(UsdcAxelar::TICKER);
        maybe_visit_on_ticker_err::<Atom, LeaseGroup>(Atom::BANK_SYMBOL);
        maybe_visit_on_ticker_err::<Atom, LeaseGroup>(Nls::TICKER);
        maybe_visit_on_ticker_err::<Atom, LeaseGroup>(Nls::BANK_SYMBOL);
        maybe_visit_on_ticker_err::<Atom, LeaseGroup>(UsdcAxelar::BANK_SYMBOL);
        maybe_visit_on_ticker_err::<Dydx, LeaseGroup>(Dydx::BANK_SYMBOL);
    }

    #[test]
    fn maybe_visit_on_bank_symbol() {
        maybe_visit_on_bank_symbol_impl::<Atom, LeaseGroup>();
        maybe_visit_on_bank_symbol_impl::<StAtom, LeaseGroup>();
        maybe_visit_on_bank_symbol_impl::<Dydx, LeaseGroup>();
        maybe_visit_on_bank_symbol_err::<UsdcAxelar, LeaseGroup>(UsdcAxelar::BANK_SYMBOL);
        maybe_visit_on_bank_symbol_err::<Atom, LeaseGroup>(Atom::TICKER);
        maybe_visit_on_bank_symbol_err::<Atom, LeaseGroup>(UsdcAxelar::TICKER);
        maybe_visit_on_bank_symbol_err::<Atom, LeaseGroup>(Nls::BANK_SYMBOL);
        maybe_visit_on_bank_symbol_err::<Atom, LeaseGroup>(Nls::TICKER);
        maybe_visit_on_bank_symbol_err::<Dydx, LeaseGroup>(Dydx::TICKER);
    }
}
