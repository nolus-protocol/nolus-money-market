use super::{AnyVisitor, Symbol, SymbolStatic};

pub trait Group: PartialEq {
    const DESCR: SymbolStatic;

    fn maybe_visit_on_ticker<V>(symbol: Symbol, visitor: V) -> MaybeAnyVisitResult<V>
    where
        Self: Sized,
        V: AnyVisitor;

    fn maybe_visit_on_bank_symbol<V>(bank_symbol: Symbol, visitor: V) -> MaybeAnyVisitResult<V>
    where
        Self: Sized,
        V: AnyVisitor;
}

pub type MaybeAnyVisitResult<V> =
    Result<Result<<V as AnyVisitor>::Output, <V as AnyVisitor>::Error>, V>;
