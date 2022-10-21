use serde::{Deserialize, Serialize};

use finance::currency::{AnyVisitor, Currency, Group, MaybeAnyVisitResult, Symbol, SymbolStatic};

use crate::{lpn::Usdc, SingleVisitorAdapter};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default, Serialize, Deserialize)]
pub struct Atom {}
impl Currency for Atom {
    const TICKER: SymbolStatic = "ATOM";
    const BANK_SYMBOL: SymbolStatic = "ibc/TBDatom";
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default, Serialize, Deserialize)]
pub struct Osmo {}
impl Currency for Osmo {
    const TICKER: SymbolStatic = "OSMO";
    const BANK_SYMBOL: SymbolStatic = "ibc/TBDosmo";
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default, Serialize, Deserialize)]
pub struct Weth {}
impl Currency for Weth {
    const TICKER: SymbolStatic = "WETH";
    const BANK_SYMBOL: SymbolStatic = "ibc/TBDweth";
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default, Serialize, Deserialize)]
pub struct Wbtc {}
impl Currency for Wbtc {
    const TICKER: SymbolStatic = "WBTC";
    const BANK_SYMBOL: SymbolStatic = "ibc/TBDwbtc";
}

pub struct LeaseGroup {}

impl Group for LeaseGroup {
    const DESCR: SymbolStatic = "lease";

    fn maybe_visit_on_ticker<V>(ticker: Symbol, visitor: V) -> MaybeAnyVisitResult<V>
    where
        V: AnyVisitor,
    {
        use finance::currency::maybe_visit_on_ticker as maybe_visit;
        let v: SingleVisitorAdapter<_> = visitor.into();
        maybe_visit::<Atom, _>(ticker, v)
            .or_else(|v| maybe_visit::<Osmo, _>(ticker, v))
            .or_else(|v| maybe_visit::<Weth, _>(ticker, v))
            .or_else(|v| maybe_visit::<Wbtc, _>(ticker, v))
            // TODO REMOVE once migrate off the single currency version
            .or_else(|v| maybe_visit::<Usdc, _>(ticker, v))
            .map_err(|v| v.0)
    }

    fn maybe_visit_on_bank_symbol<V>(bank_symbol: Symbol, visitor: V) -> MaybeAnyVisitResult<V>
    where
        Self: Sized,
        V: AnyVisitor,
    {
        use finance::currency::maybe_visit_on_bank_symbol as maybe_visit;
        let v: SingleVisitorAdapter<_> = visitor.into();
        maybe_visit::<Atom, _>(bank_symbol, v)
            .or_else(|v| maybe_visit::<Osmo, _>(bank_symbol, v))
            .or_else(|v| maybe_visit::<Weth, _>(bank_symbol, v))
            .or_else(|v| maybe_visit::<Wbtc, _>(bank_symbol, v))
            // TODO REMOVE once migrate off the single currency version
            .or_else(|v| maybe_visit::<Usdc, _>(bank_symbol, v))
            .map_err(|v| v.0)
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

    use super::{Atom, LeaseGroup, Usdc, Wbtc, Weth};

    #[test]
    fn maybe_visit_on_ticker() {
        maybe_visit_on_ticker_impl::<Atom, LeaseGroup>();
        maybe_visit_on_ticker_impl::<Osmo, LeaseGroup>();
        maybe_visit_on_ticker_impl::<Weth, LeaseGroup>();
        maybe_visit_on_ticker_impl::<Wbtc, LeaseGroup>();
        maybe_visit_on_ticker_impl::<Usdc, LeaseGroup>(); // TODO REMOVE once migrate off the single currency version
        maybe_visit_on_ticker_err::<Atom, LeaseGroup>(Atom::BANK_SYMBOL);
        maybe_visit_on_ticker_err::<Atom, LeaseGroup>(Nls::TICKER);
        maybe_visit_on_ticker_err::<Atom, LeaseGroup>(Usdc::BANK_SYMBOL);
    }

    #[test]
    fn maybe_visit_on_bank_symbol() {
        maybe_visit_on_bank_symbol_impl::<Atom, LeaseGroup>();
        maybe_visit_on_bank_symbol_impl::<Osmo, LeaseGroup>();
        maybe_visit_on_bank_symbol_impl::<Weth, LeaseGroup>();
        maybe_visit_on_bank_symbol_impl::<Wbtc, LeaseGroup>();
        maybe_visit_on_bank_symbol_impl::<Usdc, LeaseGroup>(); // TODO REMOVE once migrate off the single currency version
        maybe_visit_on_bank_symbol_err::<Atom, LeaseGroup>(Atom::TICKER);
        maybe_visit_on_bank_symbol_err::<Atom, LeaseGroup>(Usdc::TICKER);
        maybe_visit_on_bank_symbol_err::<Atom, LeaseGroup>(Nls::BANK_SYMBOL);
    }
}
