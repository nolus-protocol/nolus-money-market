use serde::{Deserialize, Serialize};

use sdk::schemars::{self, JsonSchema};

use crate::{
    currency::{AnyVisitor, Group, MaybeAnyVisitResult},
    Matcher, SymbolSlice,
};
#[cfg(dex = "osmosis")]
pub(crate) mod osmosis;

#[derive(Clone, PartialEq, Eq, JsonSchema, Serialize, Deserialize)]
#[cfg_attr(any(test, feature = "testing"), derive(Debug))]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct LeaseGroup {}

impl Group for LeaseGroup {
    const DESCR: &'static str = "lease";

    fn maybe_visit<M, V>(matcher: &M, symbol: &SymbolSlice, visitor: V) -> MaybeAnyVisitResult<V>
    where
        M: Matcher + ?Sized,
        V: AnyVisitor,
    {
        #[cfg(dex = "osmosis")]
        osmosis::maybe_visit(matcher, symbol, visitor)
    }
}
