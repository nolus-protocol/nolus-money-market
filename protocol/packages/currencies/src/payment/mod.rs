use currency::{AnyVisitor, Group, Matcher, MaybeAnyVisitResult, SymbolSlice};
use sdk::schemars::{self, JsonSchema};

use super::{lease::LeaseGroup, lpn::Lpns, native::Native};

mod osmosis_tests;

#[derive(Debug, Clone, Copy, PartialEq, Eq, JsonSchema)]
pub struct PaymentGroup {}

impl Group for PaymentGroup {
    const DESCR: &'static str = "payment";

    fn maybe_visit<M, V>(matcher: &M, symbol: &SymbolSlice, visitor: V) -> MaybeAnyVisitResult<V>
    where
        M: Matcher + ?Sized,
        V: AnyVisitor,
    {
        LeaseGroup::maybe_visit(matcher, symbol, visitor)
            .or_else(|v| Lpns::maybe_visit(matcher, symbol, v))
            .or_else(|v| Native::maybe_visit(matcher, symbol, v))
    }
}
