use serde::{Deserialize, Serialize};

use sdk::schemars::{self, JsonSchema};

use currency::{AnyVisitor, Group, Matcher, MaybeAnyVisitResult, SymbolSlice};

use super::{lease::LeaseGroup, lpn::Lpns, native::Native};

pub use self::only::PaymentOnlyGroup;

#[cfg(feature = "testing")]
pub use testing::*;

mod only;
#[cfg(feature = "testing")]
mod testing;

#[derive(PartialEq, Eq, Clone, Copy, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
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
            .or_else(|v| PaymentOnlyGroup::maybe_visit(matcher, symbol, v))
    }
}
