use serde::{Deserialize, Serialize};

use sdk::schemars::{self, JsonSchema};

use currency::{AnyVisitor, Group, Matcher, MaybeAnyVisitResult, SymbolSlice};

#[cfg(dex = "astroport")]
pub(crate) mod astroport;

#[cfg(all(not(dex = "astroport"), dex = "osmosis"))]
pub(crate) mod osmosis;

#[derive(Clone, PartialEq, Eq, JsonSchema, Serialize, Deserialize)]
#[cfg_attr(any(test, feature = "testing"), derive(Debug))]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct LeaseGroup {}

impl Group for LeaseGroup {
    const DESCR: &'static str = "lease";

    #[cfg(dex = "astroport")]
    fn maybe_visit<M, V>(matcher: &M, symbol: &SymbolSlice, visitor: V) -> MaybeAnyVisitResult<V>
    where
        M: Matcher + ?Sized,
        V: AnyVisitor,
    {
        astroport::maybe_visit(matcher, symbol, visitor)
    }

    #[cfg(all(not(dex = "astroport"), dex = "osmosis"))]
    fn maybe_visit<M, V>(matcher: &M, symbol: &SymbolSlice, visitor: V) -> MaybeAnyVisitResult<V>
    where
        M: Matcher + ?Sized,
        V: AnyVisitor,
    {
        osmosis::maybe_visit(matcher, symbol, visitor)
    }
}
