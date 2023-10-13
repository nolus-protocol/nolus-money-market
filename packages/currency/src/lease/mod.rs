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
        use crate::currency::maybe_visit_any as maybe_visit;
        #[cfg(dex = "osmosis")]
        {
            use osmosis::*;
            maybe_visit::<_, Atom, _>(matcher, symbol, visitor)
                .or_else(|visitor| maybe_visit::<_, StAtom, _>(matcher, symbol, visitor))
                .or_else(|visitor| maybe_visit::<_, Osmo, _>(matcher, symbol, visitor))
                .or_else(|visitor| maybe_visit::<_, StOsmo, _>(matcher, symbol, visitor))
                .or_else(|visitor| maybe_visit::<_, Weth, _>(matcher, symbol, visitor))
                .or_else(|visitor| maybe_visit::<_, Wbtc, _>(matcher, symbol, visitor))
                .or_else(|visitor| maybe_visit::<_, Akt, _>(matcher, symbol, visitor))
                .or_else(|visitor| maybe_visit::<_, Axl, _>(matcher, symbol, visitor))
                .or_else(|visitor| maybe_visit::<_, QAtom, _>(matcher, symbol, visitor))
                .or_else(|visitor| maybe_visit::<_, StkAtom, _>(matcher, symbol, visitor))
                .or_else(|visitor| maybe_visit::<_, Strd, _>(matcher, symbol, visitor))
                .or_else(|visitor| maybe_visit::<_, Inj, _>(matcher, symbol, visitor))
                .or_else(|visitor| maybe_visit::<_, Secret, _>(matcher, symbol, visitor))
                .or_else(|visitor| maybe_visit::<_, Stars, _>(matcher, symbol, visitor))
                .or_else(|visitor| maybe_visit::<_, Cro, _>(matcher, symbol, visitor))
                .or_else(|visitor| maybe_visit::<_, Juno, _>(matcher, symbol, visitor))
                .or_else(|visitor| maybe_visit::<_, Evmos, _>(matcher, symbol, visitor))
                .or_else(|visitor| maybe_visit::<_, Mars, _>(matcher, symbol, visitor))
        }
    }
}
