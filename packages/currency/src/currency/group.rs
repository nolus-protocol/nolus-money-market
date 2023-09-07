use serde::{de::DeserializeOwned, Serialize};

use crate::{Currency, MaybeVisitResult, SingleVisitor, SymbolSlice};

use super::{matcher::Matcher, AnyVisitor, AnyVisitorResult, SymbolStatic};

pub trait Group: PartialEq {
    const DESCR: SymbolStatic;

    fn maybe_visit<M, V>(matcher: M, symbol: &SymbolSlice, visitor: V) -> MaybeAnyVisitResult<V>
    where
        Self: Sized,
        M: Matcher,
        V: AnyVisitor;
}

pub type MaybeAnyVisitResult<V> = Result<AnyVisitorResult<V>, V>;

pub(crate) fn maybe_visit_any<M, C, V>(
    matcher: M,
    symbol: &SymbolSlice,
    visitor: V,
) -> MaybeAnyVisitResult<V>
where
    M: Matcher,
    C: Currency + Serialize + DeserializeOwned,
    V: AnyVisitor,
{
    if matcher.match_::<C>(symbol) {
        Ok(visitor.on::<C>())
    } else {
        Err(visitor)
    }
}

pub(crate) fn maybe_visit<M, C, V>(
    matcher: M,
    symbol: &SymbolSlice,
    visitor: V,
) -> MaybeVisitResult<C, V>
where
    M: Matcher,
    C: Currency,
    V: SingleVisitor<C>,
{
    if matcher.match_::<C>(symbol) {
        Ok(visitor.on())
    } else {
        Err(visitor)
    }
}
