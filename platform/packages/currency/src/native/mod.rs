use crate::{
    currency::{AnyVisitor, Group, MaybeAnyVisitResult},
    Matcher, SymbolSlice,
};

#[cfg(dex = "osmosis")]
pub(crate) mod osmosis;

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
        use crate::maybe_visit_any as maybe_visit;
        #[cfg(dex = "osmosis")]
        {
            use osmosis::Nls;
            maybe_visit::<_, Nls, _>(matcher, symbol, visitor)
        }
    }
}
