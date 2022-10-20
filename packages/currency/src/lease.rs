use serde::{Deserialize, Serialize};

use finance::currency::{
    AnyVisitor, Currency, Group, MaybeAnyVisitResult, Member, Symbol, SymbolStatic,
};

use crate::{lpn::Usdc, SingleVisitorAdapter};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default, Serialize, Deserialize)]
pub struct Atom {}
impl Currency for Atom {
    const TICKER: SymbolStatic = "ATOM";
    const BANK_SYMBOL: SymbolStatic = "ibc/TBDatom";
}
impl Member<LeaseGroup> for Atom {}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default, Serialize, Deserialize)]
pub struct Osmo {}
impl Currency for Osmo {
    const TICKER: SymbolStatic = "OSMO";
    const BANK_SYMBOL: SymbolStatic = "ibc/TBDosmo";
}
impl Member<LeaseGroup> for Osmo {}

// TODO REMOVE once migrate off the single currency version
impl Member<LeaseGroup> for Usdc {}

pub struct LeaseGroup {}

impl Group for LeaseGroup {
    const DESCR: SymbolStatic = "lease";

    fn maybe_visit_on_ticker<V>(ticker: Symbol, visitor: V) -> MaybeAnyVisitResult<Self, V>
    where
        V: AnyVisitor<Self>,
    {
        use finance::currency::maybe_visit_on_ticker as maybe_visit;
        let v: SingleVisitorAdapter<Self, _> = visitor.into();
        maybe_visit::<Atom, _>(ticker, v)
            .or_else(|v| maybe_visit::<Osmo, _>(ticker, v))
            .or_else(|v| maybe_visit::<Usdc, _>(ticker, v))
            .map_err(|v| v.0)
    }

    fn maybe_visit_on_bank_symbol<V>(
        bank_symbol: Symbol,
        visitor: V,
    ) -> MaybeAnyVisitResult<Self, V>
    where
        Self: Sized,
        V: AnyVisitor<Self>,
    {
        use finance::currency::maybe_visit_on_bank_symbol as maybe_visit;
        let v: SingleVisitorAdapter<Self, _> = visitor.into();
        maybe_visit::<Atom, _>(bank_symbol, v)
            .or_else(|v| maybe_visit::<Osmo, _>(bank_symbol, v))
            .or_else(|v| maybe_visit::<Usdc, _>(bank_symbol, v))
            .map_err(|v| v.0)
    }
}
