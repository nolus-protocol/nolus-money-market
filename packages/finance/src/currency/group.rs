use super::{AnyVisitor, AnyVisitorResult, Currency, Symbol, SymbolStatic};

pub trait Group: PartialEq {
    const DESCR: SymbolStatic;

    fn contains<C>() -> bool
    where
        C: Currency;

    fn maybe_visit_on_ticker<V>(symbol: Symbol<'_>, visitor: V) -> MaybeAnyVisitResult<V>
    where
        Self: Sized,
        V: AnyVisitor;

    fn maybe_visit_on_bank_symbol<V>(bank_symbol: Symbol<'_>, visitor: V) -> MaybeAnyVisitResult<V>
    where
        Self: Sized,
        V: AnyVisitor;
}

pub type MaybeAnyVisitResult<V> = Result<AnyVisitorResult<V>, V>;
