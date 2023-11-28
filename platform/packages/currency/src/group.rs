use std::fmt::Debug;

use sdk::schemars::JsonSchema;

use crate::SymbolSlice;

use super::{matcher::Matcher, AnyVisitor, AnyVisitorResult};

pub trait Group: Debug + Copy + Eq + JsonSchema + 'static {
    const DESCR: &'static str;

    fn maybe_visit<M, V>(matcher: &M, symbol: &SymbolSlice, visitor: V) -> MaybeAnyVisitResult<V>
    where
        M: Matcher + ?Sized,
        V: AnyVisitor;
}

pub type MaybeAnyVisitResult<V> = Result<AnyVisitorResult<V>, V>;
