use crate::visitor::{BankSymbolVisitor, DexSymbolVisitor, GeneralizedVisitorExt, TickerVisitor};

use super::{AnyVisitor, AnyVisitorResult, Symbol, SymbolStatic};

pub trait Group: PartialEq {
    const DESCR: SymbolStatic;

    fn maybe_visit_on_by_ref<GV, V>(generalized_visitor: &GV, visitor: V) -> MaybeAnyVisitResult<V>
    where
        GV: GeneralizedVisitorExt,
        V: AnyVisitor;
}

pub trait GroupExt: Group {
    fn maybe_visit_on<GV, V>(generalized_visitor: GV, visitor: V) -> MaybeAnyVisitResult<V>
    where
        GV: GeneralizedVisitorExt,
        V: AnyVisitor,
    {
        Self::maybe_visit_on_by_ref(&generalized_visitor, visitor)
    }

    fn maybe_visit_on_ticker<V>(ticker: Symbol<'_>, visitor: V) -> MaybeAnyVisitResult<V>
    where
        V: AnyVisitor,
    {
        Self::maybe_visit_on(TickerVisitor::new(ticker), visitor)
    }

    fn maybe_visit_on_bank_symbol<V>(bank_symbol: Symbol<'_>, visitor: V) -> MaybeAnyVisitResult<V>
    where
        V: AnyVisitor,
    {
        Self::maybe_visit_on(BankSymbolVisitor::new(bank_symbol), visitor)
    }

    fn maybe_visit_on_dex_symbol<V>(dex_symbol: Symbol<'_>, visitor: V) -> MaybeAnyVisitResult<V>
    where
        V: AnyVisitor,
    {
        Self::maybe_visit_on(DexSymbolVisitor::new(dex_symbol), visitor)
    }
}

impl<T> GroupExt for T where T: Group {}

pub type MaybeAnyVisitResult<V> = Result<AnyVisitorResult<V>, V>;
