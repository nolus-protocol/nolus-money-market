use currency::{AnyVisitor, Matcher, MaybeAnyVisitResult, SymbolSlice};
use sdk::schemars;
#[cfg(feature = "testing")]
pub use testing_currencies::*;

use crate::{define_currency, define_symbol};

// Resources:
// 1. Symbol hashes are computed using the SHA256 Hash Generator https://coding.tools/sha256
// 2. Currencies that come from Axelar are documented at https://docs.axelar.dev/resources
// 3. IBC routes from https://github.com/Nolus-Protocol/Wiki/blob/main/testnet-rila/currencies.json

define_symbol! {
    ATOM {
        ["net_dev"]: {
            // full ibc route: transfer/channel-116/transfer/channel-1/uatom
            bank: "ibc/59BA0C7FDC7C3CDA4C777EDEC5572C762B68DDCC9FD253BC12B6F5676395157E",
            // full ibc route: transfer/channel-1/uatom
            dex: "ibc/C4CFF46FD6DE35CA4CF4CE031E643C8FDC9BA4B99AE598E9B0ED98FE3A2319F9",
        },
        ["net_test"]: {
            // full ibc route: transfer/channel-1990/transfer/channel-1/uatom
            bank: "ibc/FB644CC503C21C67F89C111B25362AA71CEE60E803211AC939DEBF820109660C",
            // full ibc route: transfer/channel-1/uatom
            dex: "ibc/C4CFF46FD6DE35CA4CF4CE031E643C8FDC9BA4B99AE598E9B0ED98FE3A2319F9",
        },
        ["net_main"]: {
            // full ibc route: transfer/channel-3839/transfer/channel-1/uatom
            bank: "ibc/74329D0B6EAA37AD07FF44EC40D998357D1478C504AB5A9C91C3F42F1078A226",
            // full ibc route: transfer/channel-1/uatom
            dex: "ibc/C4CFF46FD6DE35CA4CF4CE031E643C8FDC9BA4B99AE598E9B0ED98FE3A2319F9",
        },
    }
}
define_currency!(Atom, ATOM);

define_symbol! {
    ST_ATOM {
        ["net_dev", "net_test"]: {
            // full ibc route: transfer/channel-?/transfer/channel-?/stuatom
            bank: "ibc/NA_ST_ATOM",
            // full ibc route: transfer/channel-?/stuatom
            dex: "ibc/NA_ST_ATOM_DEX",
        },
        ["net_main"]: {
            // full ibc route: transfer/channel-3839/transfer/channel-8/stuatom
            bank: "ibc/FFE21A5F04A89CD5E61A616EEE11A646D5DCF0A8DB60523F79C5ED28DA2642FA",
            // full ibc route: transfer/channel-8/stuatom
            dex: "ibc/B7864B03E1B9FD4F049243E92ABD691586F682137037A9F3FCA5222815620B3C",
        },
    }
}
define_currency!(StAtom, ST_ATOM);

define_symbol! {
    NTRN {
        ["net_dev"]: {
            // full ibc route: transfer/channel-116/untrn
            bank: "ibc/AED28B54A45FBE585FD81FF8571F729F87EF3CFD5DBBD1CAFED9E1AA20BFD983",
            dex: "untrn",
        },
        ["net_test"]: {
            // full ibc route: transfer/channel-1990/untrn
            bank: "ibc/712F900E327780AAB33B9204DB5257FB1D6FACCF9CD7B70A0EFB31ED4C1255C4",
            dex: "untrn",
        },
        ["net_main"]: {
            // full ibc route: transfer/channel-3839/untrn
            bank: "ibc/3D6BC6E049CAEB905AC97031A42800588C58FB471EBDC7A3530FFCD0C3DC9E09",
            dex: "untrn",
        },
    }
}
define_currency!(Ntrn, NTRN);

define_symbol! {
    DYDX {
        ["net_dev", "net_test"]: {
            // full ibc route: transfer/channel-?/transfer/channel-?/adydx
            bank: "ibc/NA_DYDX",
            // full ibc route: transfer/channel-?/adydx
            dex: "ibc/NA_DYDX_DEX",
        },
        ["net_main"]: {
            // full ibc route: transfer/channel-3839/transfer/channel-48/adydx
            bank: "ibc/6DF8CF5C976851D152E2C7270B0AB25C4F9D64C0A46513A68D6CBB2662A98DF4",
            // full ibc route: transfer/channel-48/adydx
            dex: "ibc/2CB87BCE0937B1D1DFCEE79BE4501AAF3C265E923509AEAC410AD85D27F35130",
        },
    }
}
define_currency!(Dydx, DYDX);

#[cfg(feature = "testing")]
mod testing_currencies {
    use sdk::schemars;

    use crate::{define_currency, define_symbol};

    define_symbol! {
        TEST_C1 {
            ["net_dev", "net_test", "net_main"]: {
                bank: "ibc/test_currency_1",
                dex: "ibc/test_currency_1_dex",
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
