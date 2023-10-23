use crate::{
    currency::{AnyVisitor, Group, MaybeAnyVisitResult},
    Matcher, SymbolSlice,
};

#[cfg(dex = "osmosis")]
pub(crate) mod osmosis;

#[cfg(dex = "osmosis")]
pub type Nls = osmosis::Nls;

#[derive(Clone, PartialEq, Eq)]
#[cfg_attr(any(test, feature = "testing"), derive(Debug))]
pub struct Native {}
impl Group for Native {
    const DESCR: &'static str = "native";

    fn maybe_visit<M, V>(matcher: &M, symbol: &SymbolSlice, visitor: V) -> MaybeAnyVisitResult<V>
    where
        M: Matcher + ?Sized,
        V: AnyVisitor,
    {
        crate::maybe_visit_any::<_, Nls, _>(matcher, symbol, visitor)
    }
}
