use serde::{Deserialize, Serialize};

use finance::currency::{
    AnyVisitor, Currency, Group, MaybeAnyVisitResult, Member, Symbol, SymbolStatic,
};

use crate::SingleVisitorAdapter;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default, Serialize, Deserialize)]
pub struct Usdc;
impl Currency for Usdc {
    const TICKER: SymbolStatic = "USDC";
    const BANK_SYMBOL: SymbolStatic = "ibc/TBDusdc";
}
impl Member<Lpns> for Usdc {}

pub struct Lpns {}
impl Group for Lpns {
    const DESCR: SymbolStatic = "lpns";

    fn maybe_visit_on_ticker<V>(ticker: Symbol, visitor: V) -> MaybeAnyVisitResult<Self, V>
    where
        V: AnyVisitor<Self>,
    {
        use finance::currency::maybe_visit_on_ticker as maybe_visit;
        let v: SingleVisitorAdapter<Self, _> = visitor.into();
        maybe_visit::<Usdc, _>(ticker, v).map_err(|v| v.0)
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
        maybe_visit::<Usdc, _>(bank_symbol, v).map_err(|v| v.0)
    }
}
