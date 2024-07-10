use currency::{AnyVisitor, Group, Matcher, MaybeAnyVisitResult, SymbolSlice};

pub use impl_mod::Nls;

#[cfg(not(feature = "testing"))]
use r#impl as impl_mod;
#[cfg(feature = "testing")]
use testing as impl_mod;

#[cfg(not(feature = "testing"))]
mod r#impl;
#[cfg(feature = "testing")]
mod testing;

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
