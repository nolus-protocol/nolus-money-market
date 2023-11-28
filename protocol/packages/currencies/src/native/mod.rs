use currency::{AnyVisitor, Group, Matcher, MaybeAnyVisitResult, SymbolSlice};

#[cfg(feature = "astroport")]
use self::astroport as impl_mod;
#[cfg(feature = "osmosis")]
use self::osmosis as impl_mod;

#[cfg(feature = "astroport")]
pub(crate) mod astroport;
#[cfg(feature = "osmosis")]
pub(crate) mod osmosis;

pub type Nls = impl_mod::Nls;

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
        currency::maybe_visit_any::<_, Nls, _>(matcher, symbol, visitor)
    }
}
