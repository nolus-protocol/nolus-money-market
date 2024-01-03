use currency::{AnyVisitor, Matcher, MaybeAnyVisitResult, SymbolSlice};
use sdk::schemars;

use crate::{
    define_currency, define_symbol,
    ibc::macros::{bank_symbol, dex_native_symbol, dex_symbol},
};

// Resources:
// 1. Symbol hashes are computed using the SHA256 Hash Generator https://coding.tools/sha256
// 2. Currencies that come from Axelar are documented at https://docs.axelar.dev/resources
// 3. IBC routes from https://github.com/Nolus-Protocol/Wiki/blob/main/testnet-rila/currencies.json

define_symbol! {
    ATOM {
        ["net_dev", "net_test"]: {
            bank: bank_symbol!([12], "atom"),
            dex: dex_symbol!([12], "atom"),
        },
        ["net_main"]: {
            bank: bank_symbol!([0], "uatom"),
            dex: dex_symbol!([0], "uatom"),
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
            bank: bank_symbol!([326], "stuatom"),
            dex: dex_symbol!([326], "stuatom"),
        },
    }
}
define_currency!(StAtom, ST_ATOM);

define_symbol! {
    OSMO {
        ["net_dev", "net_test", "net_main"]: {
            bank: bank_symbol!([], "uosmo"),
            dex: dex_native_symbol!("uosmo"),
        },
    }
}
define_currency!(Osmo, OSMO);

define_symbol! {
    ST_OSMO {
        ["net_dev", "net_test"]: {
            bank: crate::symbols_macro::BankSymbol("ibc/NA_ST_OSMO"),
            dex: crate::symbols_macro::DexSymbol("ibc/NA_ST_OSMO_DEX"),
        },
        ["net_main"]: {
            bank: bank_symbol!([326], "stuosmo"),
            dex: dex_symbol!([326], "stuosmo"),
        },
    }
}
define_currency!(StOsmo, ST_OSMO);

define_symbol! {
    WETH {
        ["net_dev", "net_test"]: {
            bank: bank_symbol!([3], "eth-wei"),
            dex: dex_symbol!([3], "eth-wei"),
        },
        ["net_main"]: {
            bank: bank_symbol!([208], "weth-wei"),
            dex: dex_symbol!([208], "weth-wei"),
        },
    }
}
define_currency!(Weth, WETH);

define_symbol! {
    WBTC {
        ["net_dev", "net_test"]: {
            bank: bank_symbol!([3], "btc-satoshi"),
            dex: dex_symbol!([3], "btc-satoshi"),
        },
        ["net_main"]: {
            bank: bank_symbol!([208], "wbtc-satoshi"),
            dex: dex_symbol!([208], "wbtc-satoshi"),
        },
    }
}
define_currency!(Wbtc, WBTC);

define_symbol! {
    AKT {
        ["net_dev", "net_test"]: {
            bank: bank_symbol!([73], "uakt"),
            dex: dex_symbol!([73], "uakt"),
        },
        ["net_main"]: {
            bank: bank_symbol!([1], "uakt"),
            dex: dex_symbol!([1], "uakt"),
        },
    }
}
define_currency!(Akt, AKT);

define_symbol! {
    AXL {
        ["net_dev", "net_test"]: {
            bank: crate::symbols_macro::BankSymbol("ibc/NA_AXL"),
            dex: crate::symbols_macro::DexSymbol("ibc/NA_AXL_DEX"),
        },
        ["net_main"]: {
            bank: bank_symbol!([208], "uaxl"),
            dex: dex_symbol!([208], "uaxl"),
        }
    }
}
define_currency!(Axl, AXL);

define_symbol! {
    Q_ATOM {
        ["net_dev", "net_test"]: {
            bank: crate::symbols_macro::BankSymbol("ibc/NA_Q_ATOM"),
            dex: crate::symbols_macro::DexSymbol("ibc/NA_Q_ATOM_DEX"),
        },
        ["net_main"]: {
            bank: bank_symbol!([522], "uqatom"),
            dex: dex_symbol!([522], "uqatom"),
        },
    }
}
define_currency!(QAtom, Q_ATOM);

define_symbol! {
    STK_ATOM {
        ["net_dev", "net_test"]: {
            bank: crate::symbols_macro::BankSymbol("ibc/NA_STK_ATOM"),
            dex: crate::symbols_macro::DexSymbol("ibc/NA_STK_ATOM_DEX"),
        },
        ["net_main"]: {
            bank: bank_symbol!([4], "stk/uatom"),
            dex: dex_symbol!([4], "stk/uatom"),
        },
    }
}
define_currency!(StkAtom, STK_ATOM);

define_symbol! {
    STRD {
        ["net_dev", "net_test"]: {
            bank: crate::symbols_macro::BankSymbol("ibc/NA_STRD"),
            dex: crate::symbols_macro::DexSymbol("ibc/NA_STRD_DEX"),
        },
        ["net_main"]: {
            bank: bank_symbol!([326], "ustrd"),
            dex: dex_symbol!([326], "ustrd"),
        },
    }
}
define_currency!(Strd, STRD);

define_symbol! {
    INJ {
        ["net_dev", "net_test"]: {
            bank: crate::symbols_macro::BankSymbol("ibc/NA_INJ"),
            dex: crate::symbols_macro::DexSymbol("ibc/NA_INJ_DEX"),
        },
        ["net_main"]: {
            bank: bank_symbol!([122], "inj"),
            dex: dex_symbol!([122], "inj"),
        },
    }
}
define_currency!(Inj, INJ);

define_symbol! {
    SCRT {
        ["net_dev", "net_test"]: {
            bank: crate::symbols_macro::BankSymbol("ibc/NA_SCRT"),
            dex: crate::symbols_macro::DexSymbol("ibc/NA_SCRT_DEX"),
        },
        ["net_main"]: {
            bank: bank_symbol!([88], "uscrt"),
            dex: dex_symbol!([88], "uscrt"),
        },
    }
}
define_currency!(Secret, SCRT);

define_symbol! {
    STARS {
        ["net_dev", "net_test"]: {
            bank: crate::symbols_macro::BankSymbol("ibc/NA_STARS"),
            dex: crate::symbols_macro::DexSymbol("ibc/NA_STARS_DEX"),
        },
        ["net_main"]: {
            bank: bank_symbol!([75], "ustars"),
            dex: dex_symbol!([75], "ustars"),
        },
    }
}
define_currency!(Stars, STARS);

define_symbol! {
    CRO {
        ["net_dev", "net_test"]: {
            bank: crate::symbols_macro::BankSymbol("ibc/NA_CRO"),
            dex: crate::symbols_macro::DexSymbol("ibc/NA_CRO_DEX"),
        },
        ["net_main"]: {
            bank: bank_symbol!([5], "basecro"),
            dex: dex_symbol!([5], "basecro"),
        },
    }
}
define_currency!(Cro, CRO);

define_symbol! {
    JUNO {
        ["net_dev", "net_test"]: {
            bank: bank_symbol!([1], "ujunox"),
            dex: dex_symbol!([1], "ujunox"),
        },
        ["net_main"]: {
            bank: bank_symbol!([42], "ujuno"),
            dex: dex_symbol!([42], "ujuno"),
        },
    }
}
define_currency!(Juno, JUNO);

define_symbol! {
    EVMOS {
        ["net_dev", "net_test"]: {
            bank: crate::symbols_macro::BankSymbol("ibc/NA_EVMOS"),
            dex: crate::symbols_macro::DexSymbol("ibc/NA_EVMOS_DEX"),
        },
        ["net_main"]: {
            bank: bank_symbol!([204], "aevmos"),
            dex: dex_symbol!([204], "aevmos"),
        },
    }
}
define_currency!(Evmos, EVMOS);

define_symbol! {
    MARS {
        ["net_dev", "net_test"]: {
            bank: bank_symbol!([24], "umars"),
            dex: dex_symbol!([24], "umars"),
        },
        ["net_main"]: {
            bank: bank_symbol!([557], "umars"),
            dex: dex_symbol!([557], "umars"),
        },
    }
}
define_currency!(Mars, MARS);

define_symbol! {
    TIA {
        ["net_dev", "net_test"]: {
            bank: crate::symbols_macro::BankSymbol("ibc/NA_TIA"),
            dex: crate::symbols_macro::DexSymbol("ibc/NA_TIA_DEX"),
        },
        ["net_main"]: {
            bank: bank_symbol!([6994], "utia"),
            dex: dex_symbol!([6994], "utia"),
        },
    }
}
define_currency!(Tia, TIA);

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
    maybe_visit::<_, Atom, _>(matcher, symbol, visitor)
        .or_else(|visitor| maybe_visit::<_, StAtom, _>(matcher, symbol, visitor))
        .or_else(|visitor| maybe_visit::<_, Osmo, _>(matcher, symbol, visitor))
        .or_else(|visitor| maybe_visit::<_, StOsmo, _>(matcher, symbol, visitor))
        .or_else(|visitor| maybe_visit::<_, Weth, _>(matcher, symbol, visitor))
        .or_else(|visitor| maybe_visit::<_, Wbtc, _>(matcher, symbol, visitor))
        .or_else(|visitor| maybe_visit::<_, Akt, _>(matcher, symbol, visitor))
        .or_else(|visitor| maybe_visit::<_, Axl, _>(matcher, symbol, visitor))
        .or_else(|visitor| maybe_visit::<_, QAtom, _>(matcher, symbol, visitor))
        .or_else(|visitor| maybe_visit::<_, StkAtom, _>(matcher, symbol, visitor))
        .or_else(|visitor| maybe_visit::<_, Strd, _>(matcher, symbol, visitor))
        .or_else(|visitor| maybe_visit::<_, Inj, _>(matcher, symbol, visitor))
        .or_else(|visitor| maybe_visit::<_, Secret, _>(matcher, symbol, visitor))
        .or_else(|visitor| maybe_visit::<_, Stars, _>(matcher, symbol, visitor))
        .or_else(|visitor| maybe_visit::<_, Cro, _>(matcher, symbol, visitor))
        .or_else(|visitor| maybe_visit::<_, Juno, _>(matcher, symbol, visitor))
        .or_else(|visitor| maybe_visit::<_, Evmos, _>(matcher, symbol, visitor))
        .or_else(|visitor| maybe_visit::<_, Mars, _>(matcher, symbol, visitor))
        .or_else(|visitor| maybe_visit::<_, Tia, _>(matcher, symbol, visitor))
}

#[cfg(test)]
mod test {
    use currency::Currency;

    use crate::{
        test_impl::{
            maybe_visit_on_bank_symbol_err, maybe_visit_on_bank_symbol_impl,
            maybe_visit_on_ticker_err, maybe_visit_on_ticker_impl,
        },
        {lease::LeaseGroup, lpn::osmosis::Usdc, native::osmosis::Nls},
    };

    use super::{Atom, Osmo, StAtom, StOsmo, Tia, Wbtc, Weth};

    #[test]
    fn maybe_visit_on_ticker() {
        maybe_visit_on_ticker_impl::<Atom, LeaseGroup>();
        maybe_visit_on_ticker_impl::<StAtom, LeaseGroup>();
        maybe_visit_on_ticker_impl::<Osmo, LeaseGroup>();
        maybe_visit_on_ticker_impl::<StOsmo, LeaseGroup>();
        maybe_visit_on_ticker_impl::<Weth, LeaseGroup>();
        maybe_visit_on_ticker_impl::<Wbtc, LeaseGroup>();
        maybe_visit_on_ticker_impl::<Tia, LeaseGroup>();
        maybe_visit_on_ticker_err::<Usdc, LeaseGroup>(Usdc::TICKER);
        maybe_visit_on_ticker_err::<Atom, LeaseGroup>(Atom::BANK_SYMBOL);
        maybe_visit_on_ticker_err::<Atom, LeaseGroup>(Nls::TICKER);
        maybe_visit_on_ticker_err::<Atom, LeaseGroup>(Nls::BANK_SYMBOL);
        maybe_visit_on_ticker_err::<Atom, LeaseGroup>(Usdc::BANK_SYMBOL);
    }

    #[test]
    fn maybe_visit_on_bank_symbol() {
        maybe_visit_on_bank_symbol_impl::<Atom, LeaseGroup>();
        maybe_visit_on_bank_symbol_impl::<StAtom, LeaseGroup>();
        maybe_visit_on_bank_symbol_impl::<Osmo, LeaseGroup>();
        maybe_visit_on_bank_symbol_impl::<StOsmo, LeaseGroup>();
        maybe_visit_on_bank_symbol_impl::<Weth, LeaseGroup>();
        maybe_visit_on_bank_symbol_impl::<Wbtc, LeaseGroup>();
        maybe_visit_on_bank_symbol_impl::<Tia, LeaseGroup>();
        maybe_visit_on_bank_symbol_err::<Usdc, LeaseGroup>(Usdc::BANK_SYMBOL);
        maybe_visit_on_bank_symbol_err::<Atom, LeaseGroup>(Atom::TICKER);
        maybe_visit_on_bank_symbol_err::<Atom, LeaseGroup>(Usdc::TICKER);
        maybe_visit_on_bank_symbol_err::<Atom, LeaseGroup>(Nls::BANK_SYMBOL);
        maybe_visit_on_bank_symbol_err::<Atom, LeaseGroup>(Nls::TICKER);
    }
}
