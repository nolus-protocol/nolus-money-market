use serde::{Deserialize, Serialize};

use finance::currency::{
    self, AnyVisitor, Currency, Group, MaybeAnyVisitResult, Symbol, SymbolStatic,
};

use crate::SingleVisitorAdapter;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default, Serialize, Deserialize)]
pub struct Usdc;
impl Currency for Usdc {
    const TICKER: SymbolStatic = "USDC";
    const BANK_SYMBOL: SymbolStatic = "ibc/TBDusdc";
}

pub struct Lpns {}
impl Group for Lpns {
    const DESCR: SymbolStatic = "lpns";

    fn maybe_visit_on_ticker<V>(ticker: Symbol, visitor: V) -> MaybeAnyVisitResult<V>
    where
        V: AnyVisitor,
    {
        let v: SingleVisitorAdapter<_> = visitor.into();
        currency::maybe_visit_on_ticker::<Usdc, _>(ticker, v).map_err(|v| v.0)
    }

    fn maybe_visit_on_bank_symbol<V>(bank_symbol: Symbol, visitor: V) -> MaybeAnyVisitResult<V>
    where
        Self: Sized,
        V: AnyVisitor,
    {
        let v: SingleVisitorAdapter<_> = visitor.into();
        currency::maybe_visit_on_bank_symbol::<Usdc, _>(bank_symbol, v).map_err(|v| v.0)
    }
}
