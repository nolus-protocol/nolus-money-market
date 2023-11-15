use serde::{Deserialize, Serialize};

use sdk::schemars::{self, JsonSchema};

use currency::{AnyVisitor, Group, Matcher, MaybeAnyVisitResult, SymbolSlice};
#[cfg(dex = "osmosis")]
pub(crate) mod osmosis;

#[derive(Clone, Debug, PartialEq, Eq, JsonSchema, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct Lpns {}

impl Group for Lpns {
    const DESCR: &'static str = "lpns";

    fn maybe_visit<M, V>(matcher: &M, symbol: &SymbolSlice, visitor: V) -> MaybeAnyVisitResult<V>
    where
        M: Matcher + ?Sized,
        V: AnyVisitor,
    {
        #[cfg(dex = "osmosis")]
        osmosis::maybe_visit(matcher, symbol, visitor)
    }
}
